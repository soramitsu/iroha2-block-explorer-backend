use iroha_config::{base::WithOrigin, parameters::actual::Kura as Config};
use iroha_core::query::store::LiveQueryStore;
use nonzero_ext::nonzero;
use std::ops::Deref;
use std::time::Duration;
use std::{future::Future, num::NonZero, path::PathBuf, sync::Arc};

use crate::core::StateViewExt;
use crate::telemetry::blockchain::Metrics;
use crate::telemetry::{AverageBlockTime, Telemetry};

use super::{Error, Result, KURA_BLOCKS_IN_MEMORY};
use iroha_core::block::{CommittedBlock, ValidBlock};
use iroha_core::kura::{self, Kura};
use iroha_core::state::{State as CoreState, StateBlock, StateReadOnly, StateView, World};
use iroha_data_model::prelude::*;
use iroha_futures::supervisor::{
    spawn_os_thread_as_future, Child, OnShutdown, ShutdownSignal, Supervisor,
};
use tokio::sync::{mpsc, oneshot, watch, OwnedRwLockReadGuard, RwLock, RwLockReadGuard};

type BlockHash = HashOf<BlockHeader>;

#[derive(Clone)]
pub struct State {
    actor: mpsc::Sender<Message>,
    lock: Arc<RwLock<StateInner>>,
}

struct Actor {
    handle: mpsc::Receiver<Message>,
    telemetry: Telemetry,
    lock: Arc<RwLock<StateInner>>,
    state_extras: StateExtras,
    store_dir: PathBuf,
    shutdown_external: ShutdownSignal,
    shutdown_internal: ShutdownSignal,
    shutdown_kura_complete: Option<oneshot::Receiver<()>>,
    shutdown_state_complete: Option<oneshot::Receiver<()>>,
}

#[derive(Debug)]
enum Message {
    ConfirmHeight {
        height: usize,
    },
    InsertBlock {
        block: Arc<SignedBlock>,
        reply: oneshot::Sender<Result<()>>,
    },
}

pub struct StateGuard {
    guard: OwnedRwLockReadGuard<StateInner>,
}

struct StateInner {
    state: Arc<CoreState>,
    kura: Arc<Kura>,
}

struct StateExtras {
    metrics: IncrementalMetrics,
}

struct IncrementalMetrics {
    base: Metrics,
    block_time: AverageBlockTime,
}

impl StateGuard {
    pub fn view(&self) -> StateView<'_> {
        self.guard.state.view()
    }

    pub fn kura(&self) -> &Kura {
        &*self.guard.kura
    }
}

impl State {
    pub fn new(
        store_dir: impl Into<PathBuf>,
        telemetry: Telemetry,
        signal: ShutdownSignal,
    ) -> (
        Self,
        impl Future<Output = Result<(), iroha_futures::supervisor::Error>> + Sized,
    ) {
        let store_dir = store_dir.into();
        let (tx, rx) = mpsc::channel(128);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let mut main_sup = Supervisor::new();
        main_sup.shutdown_on_external_signal(signal.clone());

        let shutdown_internal = ShutdownSignal::new();
        let mut kura_sup = Supervisor::new();
        kura_sup.shutdown_on_external_signal(shutdown_internal);
        let (kura, child) = init_kura(
            store_dir.clone(),
            StartMode::Restore,
            kura_sup.shutdown_signal(),
        );
        kura_sup.monitor(child);
        let shutdown_kura_complete = oneshot::channel();
        let lock = Arc::new(RwLock::new(StateInner {
            state: Arc::new(init_dummy_state()),
            kura,
        }));
        let lock2 = lock.clone();
        let actor = Actor {
            handle: rx,
            telemetry,
            lock,
            store_dir,
            state_extras: StateExtras {
                metrics: IncrementalMetrics::default(),
            },
            shutdown_external: main_sup.shutdown_signal(),
            shutdown_internal: kura_sup.shutdown_signal(),
            shutdown_kura_complete: Some(shutdown_kura_complete.1),
            shutdown_state_complete: None,
        };

        main_sup.monitor(Child::new(
            tokio::spawn(spawn_os_thread_as_future(
                std::thread::Builder::new().name("kura".to_owned()),
                move || {
                    rt.block_on(async move {
                        tokio::spawn(run_sup_with_complete(kura_sup, shutdown_kura_complete.0));
                        actor.recv_loop().await
                    });
                },
            )),
            OnShutdown::Wait(Duration::from_secs(6)),
        ));

        (
            Self {
                actor: tx,
                lock: lock2,
            },
            main_sup.start(),
        )
    }

    pub async fn acquire_guard(&self) -> StateGuard {
        let guard = self.lock.clone().read_owned().await;
        StateGuard { guard }
    }

    /// Tell which latest block height in the local storage is "confirmed" to match with
    /// remote block chain.
    ///
    /// This _may_ cause state and storage re-initialisation.
    ///
    /// Future returned by this function resolves once the actor receives the message and
    /// does not wait for the entire process to finish.
    pub async fn confirm_local_height(&self, height: usize) -> Result<()> {
        self.actor.send(Message::ConfirmHeight { height }).await?;
        Ok(())
    }

    /// Insert a block into the local block chain.
    ///
    /// The block must be chained to the latest local block.
    /// If it is not, call [`Self::confirm_local_height`] first.
    ///
    /// The future resolves once the block is fully applied in the local state.
    pub async fn insert_block(&self, block: Arc<SignedBlock>) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.actor
            .send(Message::InsertBlock { block, reply: tx })
            .await?;
        rx.await?
    }
}

impl Actor {
    async fn recv_loop(mut self) {
        loop {
            tokio::select! {
                Some(msg) = self.handle.recv() => {
                    self.handle_message(msg).await
                },
                _ = self.shutdown_external.receive() => {
                    self.terminate().await
                }
            }
        }
    }

    async fn handle_message(&mut self, message: Message) {
        tracing::trace!(?message, "Handle message");
        match message {
            Message::ConfirmHeight { height } => self.confirm_height(height).await,
            Message::InsertBlock { block, reply } => {
                let result = self.insert_block((*block).clone()).await;
                let _ = reply.send(result);
            }
        }
    }

    async fn terminate(&mut self) {
        tracing::debug!("Terminating");
        self.shutdown_internal.send();
        let mut _guard = self.lock.write().await;
        tokio::join!(
            maybe_recv(self.shutdown_kura_complete.take()),
            maybe_recv(self.shutdown_state_complete.take())
        );
    }

    async fn confirm_height(&mut self, height: usize) {
        let read_guard = self.lock.read().await;
        let kura_height = read_guard.kura.blocks_count();

        if height > kura_height {
            tracing::error!(
                height,
                kura_height,
                "Received confirmed height is higher than Kura's"
            );
            return;
        }

        let state_height = read_guard.state.view().height();
        if kura_height == state_height && kura_height == height {
            // up to date
            return;
        }

        tracing::info!(height, "Restarting Kura...");

        // tell Kura to shut down
        self.shutdown_internal.send();

        // wait for all readers to cease
        drop(read_guard);
        let mut write_guard = self.lock.write().await;

        // wait Kura & State to complete their shutdown
        tokio::join!(
            maybe_recv(self.shutdown_kura_complete.take()),
            maybe_recv(self.shutdown_state_complete.take())
        );

        // dropping the state - it must hold the last Kura reference
        write_guard.state = Arc::new(init_dummy_state());

        // ensure there are no more Kura references in Explorer and we can
        // safely re-initialise it
        debug_assert_eq!(Arc::strong_count(&write_guard.kura), 1);

        // reset metrics
        self.state_extras.metrics = IncrementalMetrics::default();
        self.send_metrics().await;

        // initialise Kura

        let mut sup = Supervisor::new();
        sup.shutdown_on_external_signal(self.shutdown_external.clone());

        let (kura, child) = init_kura(
            self.store_dir.clone(),
            StartMode::RestoreOnly(height),
            sup.shutdown_signal(),
        );
        sup.monitor(child);
        write_guard.kura = kura.clone();
        let kura_complete = oneshot::channel();
        self.shutdown_kura_complete = Some(kura_complete.1);

        tokio::spawn(run_sup_with_complete(sup, kura_complete.0));

        if height > 0 {
            // replay blocks from Kura

            let genesis = write_guard
                .kura
                .get_block(nonzero!(1usize))
                .expect("storage has at least genesis");
            let (state, sup) = init_state(kura.clone(), infer_genesis_account(&*genesis).unwrap());
            let state = Arc::new(state);
            write_guard.state = state.clone();
            let state_complete = oneshot::channel();
            tokio::spawn(run_sup_with_complete(sup, state_complete.0));

            debug_assert!(self.shutdown_state_complete.is_none());
            self.shutdown_state_complete = Some(state_complete.1);

            // at this point, concurrent reads are OK
            core::mem::drop(write_guard);

            play_all_blocks(&kura, &state, &mut self.state_extras);
            self.send_metrics().await;
        }
    }

    async fn send_metrics(&self) {
        self.telemetry
            .update_blockchain_state(self.state_extras.metrics.get_metrics())
            .await;
    }

    async fn insert_block(&mut self, block: SignedBlock) -> Result<()> {
        let read_guard = self.lock.read().await;
        let kura = read_guard.kura.clone();

        let block_height: NonZero<usize> = block
            .header()
            .height()
            .try_into()
            .expect("must fit into usize");

        let kura_height = kura.blocks_count();
        let state_height = read_guard.state.view().height();
        if state_height != kura_height {
            return Err(Error::NotConfirmed);
        };

        if state_height != block_height.get() - 1 {
            return Err(Error::ReceivedBlockHeightMismatch {
                expected: NonZero::new(state_height + 1).expect("at least 1"),
                actual: block_height,
            });
        }

        if block_height.get() == 1 {
            // genesis - recreate State

            // store immediately
            let block = Arc::new(block);
            kura.store_block(block.clone());

            drop(read_guard);
            let mut write_guard = self.lock.write().await;

            let (state, sup) = init_state(kura.clone(), infer_genesis_account(&*block).unwrap());
            let state = Arc::new(state);
            // NOTE: no need to wait for previous state "shutdown" - it is a dummy
            write_guard.state = state.clone();
            let state_complete = oneshot::channel();
            tokio::spawn(run_sup_with_complete(sup, state_complete.0));

            debug_assert!(self.shutdown_state_complete.is_none());
            self.shutdown_state_complete = Some(state_complete.1);

            // at this point, concurrent reads (i.e. queries) are OK
            drop(write_guard);

            play_all_blocks(&kura, &state, &mut self.state_extras);
            self.send_metrics().await;
        } else {
            // add block to Kura & State

            let state_height = NonZero::new(state_height).expect("block > 1, therefore state >= 1");

            let local_last_block_hash = kura
                .get_block_hash(state_height)
                .expect("Kura & state are in sync");
            let block_prev_block_hash = block.header().prev_block_hash().expect("block > 1");
            if local_last_block_hash != block_prev_block_hash {
                return Err(Error::ReceivedBlockPreviousBlockHashMismatch {
                    actual_prev_block_hash: local_last_block_hash,
                    block_prev_block_hash,
                });
            }

            // OK: block is valid, apply
            let block = Arc::new(block);
            kura.store_block(block.clone());
            play_all_blocks(&kura, &read_guard.state, &mut self.state_extras);
            self.send_metrics().await;
        }

        Ok(())
    }
}

impl Default for IncrementalMetrics {
    fn default() -> Self {
        todo!()
    }
}

impl IncrementalMetrics {
    fn get_metrics(&self) -> Metrics {
        todo!()
    }

    fn insert_block_delta(&mut self, delta: Duration) {}

    fn reflect_state(&mut self, view: &impl StateReadOnly) {}
}

fn play_all_blocks(kura: &Arc<Kura>, state: &CoreState, extras: &mut StateExtras) {
    let state_height = state.view().height();
    let kura_height = kura.blocks_count();
    let mut prev_creation_time = NonZero::new(kura_height)
        .and_then(|h| kura.get_block(h))
        .map(|b| b.header().creation_time());

    for block in ((state_height + 1)..=kura_height).map(|height| {
        kura.get_block(NonZero::new(height).expect("at least 1"))
            .expect("Kura has this block")
    }) {
        let state_block = state.block(block.header());
        // FIXME: avoid ownership in play?
        let _committed = play_block((*block).clone(), state_block);

        let creation_time = block.header().creation_time();
        let delta = block.header().creation_time()
            - prev_creation_time.expect("must exist if we are in this for loop");
        extras.metrics.insert_block_delta(delta);
    }

    extras.metrics.reflect_state(&state.view());
}

fn play_block(block: SignedBlock, mut state_block: StateBlock<'_>) -> CommittedBlock {
    let valid_block = ValidBlock::validate_unchecked(block, &mut state_block).unpack(|_| {});
    let committed_block = valid_block.commit_unchecked().unpack(|_| {});
    // TODO: incorporate events
    let _events = state_block.apply_without_execution(&committed_block, vec![]);
    state_block.commit();
    committed_block
}

fn infer_genesis_account(genesis: &SignedBlock) -> Result<&AccountId> {
    let account = genesis
        .transactions()
        .next()
        .ok_or(Error::GenesisNoTransactions)?
        .authority();
    Ok(account)
}

fn init_state(kura: Arc<Kura>, genesis_account: &AccountId) -> (CoreState, Supervisor) {
    let mut sup = Supervisor::new();

    let world = {
        let account = Account::new(genesis_account.clone()).build(genesis_account);
        let domain = Domain::new(genesis_account.domain().clone()).build(&genesis_account);
        World::with([domain], [account], [])
    };

    let query_handle = {
        // TODO: adjust config?
        let cfg = iroha_config::parameters::actual::LiveQueryStore::default();
        let (handle, child) = LiveQueryStore::from_config(cfg, sup.shutdown_signal()).start();
        sup.monitor(child);
        handle
    };

    let state = CoreState::new(world, kura, query_handle);

    (state, sup)
}

fn init_dummy_state() -> CoreState {
    let world = World::with([], [], []);
    let query_handle = LiveQueryStore::dummy();
    let kura = Kura::blank_kura_for_testing();
    CoreState::new(world, kura, query_handle)
}

fn kura_config(store_dir: PathBuf) -> Config {
    Config {
        init_mode: iroha_config::kura::InitMode::Fast,
        store_dir: WithOrigin::inline(store_dir),
        blocks_in_memory: KURA_BLOCKS_IN_MEMORY,
        debug_output_new_blocks: false,
    }
}

fn init_kura(store_dir: PathBuf, mode: StartMode, signal: ShutdownSignal) -> (Arc<Kura>, Child) {
    // TODO: start mode; start thread

    let (kura, _block_count) =
        Kura::new(&kura_config(store_dir.clone())).expect("fatal: cannot initialize Kura");

    let child = Kura::start(kura.clone(), signal);

    (kura, child)
}

pub enum StartMode {
    Restore,
    /// Restore existing storage, but wipe blocks after the given height
    RestoreOnly(usize),
}

async fn maybe_recv(rx: Option<oneshot::Receiver<()>>) {
    if let Some(rx) = rx {
        rx.await;
    }
}

async fn run_sup_with_complete(sup: Supervisor, tx: oneshot::Sender<()>) {
    if let Err(err) = sup.start().await {
        tracing::error!(%err, "TODO")
    }
    let _ = tx.send(());
}
