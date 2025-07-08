use super::{Error, Result, KURA_BLOCKS_IN_MEMORY};
use crate::StateViewExt;
use iroha_config::{base::WithOrigin, parameters::actual::Kura as Config};
use iroha_core::block::{CommittedBlock, ValidBlock};
use iroha_core::kura::{self, BlockStore, Kura};
use iroha_core::query::store::LiveQueryStore;
use iroha_core::state::{
    State as CoreState, StateBlock, StateReadOnly, StateView, World, WorldReadOnly,
};
use iroha_data_model::prelude::*;
use iroha_explorer_telemetry::{blockchain::Metrics, AverageBlockTime, Telemetry};
use iroha_futures::supervisor::{
    spawn_os_thread_as_future, Child, OnShutdown, ShutdownSignal, Supervisor,
};
use mv::storage::StorageReadOnly as _;
use nonzero_ext::nonzero;
use std::ops::Deref;
use std::time::Duration;
use std::{future::Future, num::NonZero, path::PathBuf, sync::Arc};
use tokio::sync::{mpsc, oneshot, watch, OwnedRwLockReadGuard, RwLock, RwLockReadGuard};
use tracing::instrument;

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
    expect_soft_fork: bool,
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
        reply: oneshot::Sender<Result<()>>,
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
            expect_soft_fork: false,
            shutdown_external: main_sup.shutdown_signal(),
            shutdown_internal: kura_sup.shutdown_signal(),
            shutdown_kura_complete: Some(shutdown_kura_complete.1),
            shutdown_state_complete: None,
        };

        main_sup.monitor(Child::new(
            tokio::spawn(spawn_os_thread_as_future(
                std::thread::Builder::new().name("state_actor".to_owned()),
                move || {
                    rt.block_on(async move {
                        tokio::spawn(wrap_supervisor("kura", kura_sup, shutdown_kura_complete.0));
                        actor.send_metrics().await;
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
    /// The future resolves once the entire process completes
    pub async fn confirm_local_height(&self, height: usize) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.actor
            .send(Message::ConfirmHeight { height, reply: tx })
            .await?;
        rx.await?
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
                    self.terminate().await;
                    break
                }
            }
        }
        tracing::trace!("State loop exit");
    }

    async fn handle_message(&mut self, message: Message) {
        tracing::trace!(?message, "Handle message");
        match message {
            Message::ConfirmHeight { height, reply } => {
                let result = self.confirm_height(height).await;
                let _ = reply.send(result);
            }
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
        tracing::trace!("Kura & State terminated");
    }

    async fn confirm_height(&mut self, height: usize) -> Result<()> {
        let read_guard = self.lock.read().await;
        let kura_height = read_guard.kura.blocks_count();

        if self.expect_soft_fork && height == kura_height {
            tracing::debug!("Reset soft fork");
            self.expect_soft_fork = false;
            return Ok(());
        }

        if height > kura_height {
            return Err(Error::ConfirmedHeightExceedsKura {
                received: height,
                kura: kura_height,
            });
        }

        let state_height = read_guard.state.view().height();
        if kura_height == state_height && kura_height == height {
            // up to date
            return Ok(());
        }

        if kura_height == state_height && kura_height == height + 1 {
            tracing::debug!("Set soft fork");
            self.expect_soft_fork = true;
            return Ok(());
        }
        self.expect_soft_fork = false;

        tracing::info!(height, "Restarting Kura...");

        // tell Kura to shut down
        self.shutdown_internal.send();

        // wait for all readers to cease
        drop(read_guard);
        let mut write_guard = self.lock.write().await;
        tracing::trace!("All readers are dropped");

        // wait Kura & State to complete their shutdown
        tokio::join!(
            maybe_recv(self.shutdown_kura_complete.take()),
            maybe_recv(self.shutdown_state_complete.take())
        );
        tracing::trace!("Kura & State terminated");

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

        tokio::spawn(wrap_supervisor("kura", sup, kura_complete.0));

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
            tokio::spawn(wrap_supervisor("state", sup, state_complete.0));

            debug_assert!(self.shutdown_state_complete.is_none());
            self.shutdown_state_complete = Some(state_complete.1);

            // at this point, concurrent reads are OK
            core::mem::drop(write_guard);

            replay_all_blocks(&kura, &state, &mut self.state_extras);
            self.send_metrics().await;
        }

        Ok(())
    }

    async fn send_metrics(&self) {
        self.telemetry
            .try_update_blockchain_state(self.state_extras.metrics.get_metrics())
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

        if self.expect_soft_fork {
            debug_assert!(state_height > 0);
            if state_height != block_height.get() {
                return Err(Error::ReceivedBlockHeightMismatch {
                    expected: NonZero::new(state_height)
                        .expect("soft fork flag could only be set with non-empty local height"),
                    actual: block_height,
                });
            }

            // replace top block

            kura.replace_top_block(block.clone());
            let state_block = read_guard.state.block_and_revert(block.header());
            let _committed = play_block(block.to_owned(), state_block);
            if let Some(prev_block_time) = NonZero::new(block_height.get() - 1).map(|height| {
                kura.get_block(height)
                    .expect("block must be in Kura")
                    .header()
                    .creation_time()
            }) {
                // FIXME: duplication
                let delta = block
                    .header()
                    .creation_time()
                    .checked_sub(prev_block_time)
                    .expect("prev block creation time must preceed");
                self.state_extras.metrics.replace_block_delta(delta);
            }
            self.state_extras
                .metrics
                .reflect_state(&read_guard.state.view());

            self.expect_soft_fork = false;
            return Ok(());
        }

        if state_height + 1 != block_height.get() {
            return Err(Error::ReceivedBlockHeightMismatch {
                expected: NonZero::new(state_height + 1).expect("at least 1"),
                actual: block_height,
            });
        }

        if block_height.get() == 1 {
            // genesis - recreate State

            drop(read_guard);
            let mut write_guard = self.lock.write().await;

            let (state, mut sup) = init_state(kura.clone(), infer_genesis_account(&block).unwrap());
            sup.shutdown_on_external_signal(self.shutdown_internal.clone());
            let state = Arc::new(state);
            // NOTE: no need to wait for previous state "shutdown" - it is a dummy
            write_guard.state = state.clone();
            let state_complete = oneshot::channel();
            tokio::spawn(wrap_supervisor("state", sup, state_complete.0));

            debug_assert!(self.shutdown_state_complete.is_none());
            self.shutdown_state_complete = Some(state_complete.1);

            // at this point, concurrent reads (i.e. queries) are OK
            drop(write_guard);

            replay_all_blocks(&kura, &state, &mut self.state_extras);

            let state_block = state.block(block.header());
            let block: SignedBlock = play_block(block, state_block).into();
            kura.store_block(Arc::new(block));
            self.state_extras.metrics.reflect_state(&state.view());
            // TODO: update block time
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
            let state_block = read_guard.state.block(block.header());
            let block: SignedBlock = play_block(block, state_block).into();
            kura.store_block(Arc::new(block));
            self.state_extras
                .metrics
                .reflect_state(&read_guard.state.view());
            // TODO: update block time
            self.send_metrics().await;
        }

        Ok(())
    }
}

impl Default for IncrementalMetrics {
    fn default() -> Self {
        Self {
            base: <_>::default(),
            block_time: <_>::default(),
        }
    }
}

impl IncrementalMetrics {
    fn get_metrics(&self) -> Metrics {
        self.base.clone()
    }

    fn insert_block_delta(&mut self, delta: Duration) {}

    fn replace_block_delta(&mut self, delta: Duration) {}

    fn reflect_state(&mut self, view: &impl StateReadOnly) {
        self.base.block = view.height();

        // TODO: other fields
        self.base.accounts = view.world().accounts().len();
        self.base.domains = view.world().domains().len();
        self.base.assets = view.world().assets().len() + view.world().nfts().len();
    }
}

fn errors_equal(block1: &SignedBlock, block2: &SignedBlock) -> bool {
    let errs1 = block1.errors();
    let errs2 = block2.errors();
    errs1.len() == errs2.len() && errs1.zip(errs2).all(|(a, b)| a == b)
}

fn replay_all_blocks(kura: &Arc<Kura>, state: &CoreState, extras: &mut StateExtras) {
    let state_height = state.view().height();
    let kura_height = kura.blocks_count();
    let mut prev_creation_time = NonZero::new(state_height)
        .and_then(|h| kura.get_block(h))
        .map(|b| b.header().creation_time());

    for block in ((state_height + 1)..=kura_height).map(|height| {
        kura.get_block(NonZero::new(height).expect("at least 1"))
            .expect("Kura has this block")
    }) {
        let state_block = state.block(block.header());
        // FIXME: avoid ownership in play?
        let committed: SignedBlock = play_block((*block).clone(), state_block).into();

        if !errors_equal(&*block, &committed) {
            #[cfg(debug_assertions)]
            panic!("Bug: errors differ");
            #[cfg(not(debug_assertions))]
            tracing::warn!("Committed block errors differ from stored block errors");
        }

        let creation_time = block.header().creation_time();
        if let Some(prev) = prev_creation_time {
            let delta = creation_time
                .checked_sub(prev)
                .expect("prev block creation time must preceed");
            extras.metrics.insert_block_delta(delta);
        }
        prev_creation_time = Some(creation_time);
    }

    extras.metrics.reflect_state(&state.view());
}

#[instrument(fields(block = %block.header().hash(), height = block.header().height()), skip(state_block, block))]
fn play_block(block: SignedBlock, mut state_block: StateBlock<'_>) -> CommittedBlock {
    tracing::trace!("Playing block");
    let valid_block = ValidBlock::validate_unchecked(block, &mut state_block).unpack(|_| {});
    let committed_block = valid_block.commit_unchecked().unpack(|_| {});
    // TODO: incorporate events
    let _events = state_block.apply_without_execution(&committed_block, vec![]);
    tracing::trace!("Block applied");
    state_block.commit();
    tracing::trace!("Block committed");
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
    match mode {
        StartMode::Restore => {}
        StartMode::RestoreOnly(height) => {
            let store = BlockStore::new(&store_dir);
            store
                .prune(height as u64)
                .expect("fatal: cannot prune blocks");
            tracing::trace!(%height, "Pruned blocks");
        }
    }

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

#[instrument(skip(sup, tx))]
async fn wrap_supervisor(name: &str, sup: Supervisor, tx: oneshot::Sender<()>) {
    if let Err(err) = sup.start().await {
        tracing::error!(%err, "Supervisor exited with an error")
    }
    let _ = tx.send(());
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;
    use iroha_core::{block::BlockBuilder, state::WorldReadOnly, tx::AcceptedTransaction};
    use iroha_explorer_telemetry as telemetry;
    use iroha_explorer_test_utils::init_test_logger;
    use iroha_futures::supervisor;
    use iroha_primitives::time::{MockTimeHandle, TimeSource};
    use iroha_test_samples::{
        gen_account_in, ALICE_ID as ALICE, ALICE_KEYPAIR as ALICE_KEY, CARPENTER_ID as CARPENTER,
        CARPENTER_KEYPAIR as CARPENTER_KEY, PEER_KEYPAIR as PEER_KEY,
        SAMPLE_GENESIS_ACCOUNT_ID as GENESIS, SAMPLE_GENESIS_ACCOUNT_KEYPAIR as GENESIS_KEY,
    };
    use mv::storage::StorageReadOnly;
    use tempfile::TempDir;
    use tokio::{
        task::JoinHandle,
        time::{sleep, timeout},
    };
    use tracing::debug;

    const TICK: Duration = Duration::from_millis(100);
    const BLOCK_TICK: Duration = Duration::from_millis(100);
    const TELEMETRY_DEFAULT_CAPACITY: usize = 8;

    fn telemetry_factory() -> (Telemetry, mpsc::Receiver<telemetry::ActorMessage>) {
        let (tx, rx) = mpsc::channel(TELEMETRY_DEFAULT_CAPACITY);
        (Telemetry::new_dummy(tx), rx)
    }

    struct Sandbox {
        chain_id: ChainId,
        telemetry: Option<mpsc::Receiver<telemetry::ActorMessage>>,
        signal: ShutdownSignal,
        state: State,
        supervisor_task: JoinHandle<Result<(), supervisor::Error>>,
        time_handle: MockTimeHandle,
        time_source: TimeSource,
        blocks: Vec<Arc<SignedBlock>>,
    }

    impl Sandbox {
        fn new(store: impl Into<PathBuf>) -> Self {
            let (tel, tel_rx) = telemetry_factory();
            let signal = ShutdownSignal::new();

            let (state, sup_fut) = State::new(store.into(), tel, signal.clone());
            let sup_join = tokio::spawn(sup_fut);

            let (handle, source) = TimeSource::new_mock(Duration::ZERO);

            Self {
                chain_id: ChainId::from("test"),
                telemetry: Some(tel_rx),
                signal,
                state,
                supervisor_task: sup_join,
                time_handle: handle,
                time_source: source,
                blocks: <_>::default(),
            }
        }

        fn create_block<I: Instruction>(
            &mut self,
            isi: impl IntoIterator<Item = I>,
        ) -> Arc<SignedBlock> {
            let tx = TransactionBuilder::new(self.chain_id.to_owned(), GENESIS.to_owned())
                .with_instructions(isi)
                .sign(GENESIS_KEY.private_key());
            let block: SignedBlock = BlockBuilder::new_with_time_source(
                [tx].into_iter()
                    .map(AcceptedTransaction::new_unchecked)
                    .collect(),
                self.time_source.to_owned(),
            )
            .chain(0, self.blocks.last().as_ref().map(|x| x.as_ref()))
            .sign(GENESIS_KEY.private_key())
            .unpack(|_| {})
            .into();
            let block = Arc::new(block);
            self.blocks.push(block.clone());
            block
        }
    }

    macro_rules! parse {
        ($raw:expr) => {
            $raw.parse().unwrap()
        };
    }

    #[tokio::test]
    async fn start_and_shutdown() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let sandbox = Sandbox::new(store.path());

        sandbox.signal.send();

        timeout(TICK, sandbox.supervisor_task).await??;

        Ok(())
    }

    #[tokio::test]
    async fn read_from_empty_state() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let sandbox = Sandbox::new(store.path());

        let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;
        assert_eq!(guard.view().world().domains().len(), 0);
        assert_eq!(guard.kura().blocks_count(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn insert_blocks_normally() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let mut sandbox = Sandbox::new(store.path());

        let block = sandbox.create_block::<InstructionBox>([
            Register::domain(Domain::new(ALICE.domain().to_owned())).into(),
            Register::account(Account::new(ALICE.to_owned())).into(),
        ]);
        timeout(BLOCK_TICK, sandbox.state.insert_block(block.clone())).await??;

        let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;
        let view = guard.view();
        let domains: Vec<_> = view
            .world()
            .domains_iter()
            .map(|x| x.id().to_string())
            .collect();
        assert_eq!(domains, ["genesis", "wonderland"]);

        let rose = AssetDefinitionId::new(ALICE.domain().to_owned(), parse!("rose"));
        let block =
            sandbox.create_block([Register::asset_definition(AssetDefinition::numeric(rose))]);
        timeout(BLOCK_TICK, sandbox.state.insert_block(block)).await??;

        let guard2 = timeout(TICK, sandbox.state.acquire_guard()).await?;
        let view2 = guard.view();

        let assets2: Vec<_> = view2
            .world()
            .asset_definitions_iter()
            .map(|x| x.id().name().to_string())
            .collect();
        assert_eq!(assets2, ["rose"]);

        // previous data must persist
        let assets1: Vec<_> = view
            .world()
            .asset_definitions_iter()
            .map(|x| x.id().name().to_string())
            .collect();
        assert!(assets1.is_empty());

        Ok(())
    }

    async fn reinit_prep(sandbox: &mut Sandbox) -> eyre::Result<AssetId> {
        let asset_def = AssetDefinitionId::new(ALICE.domain().to_owned(), parse!("coin"));
        let block = sandbox.create_block::<InstructionBox>([
            Register::domain(Domain::new(ALICE.domain().to_owned())).into(),
            Register::account(Account::new(ALICE.to_owned())).into(),
            Register::asset_definition(AssetDefinition::numeric(asset_def.to_owned())).into(),
        ]);
        timeout(BLOCK_TICK, sandbox.state.insert_block(block.clone())).await??;

        // some more blocks
        let asset = AssetId::new(asset_def.to_owned(), ALICE.to_owned());
        for i in 0..5 {
            let block = sandbox.create_block([Mint::asset_numeric(5u32, asset.to_owned())]);
            timeout(BLOCK_TICK, sandbox.state.insert_block(block)).await??;
        }

        {
            let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;
            assert_eq!(guard.kura().blocks_count(), 6);

            let view = guard.view();
            let value = view.world().assets().get(&asset).unwrap();
            assert_eq!(value.value, Numeric::from(25u32));
        }

        Ok(asset)
    }

    #[tokio::test]
    async fn reinit_wipe() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let mut sandbox = Sandbox::new(store.path());

        reinit_prep(&mut sandbox).await?;

        // wipe!
        timeout(BLOCK_TICK, sandbox.state.confirm_local_height(0)).await?;

        {
            let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;
            assert_eq!(guard.kura().blocks_count(), 0);

            let view = guard.view();
            assert_eq!(view.world().domains().len(), 0);
            assert_eq!(view.world().accounts().len(), 0);
            assert_eq!(view.world().assets().len(), 0);
        }

        Ok(())
    }

    #[tokio::test]
    async fn reinit_rewind() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let mut sandbox = Sandbox::new(store.path());

        let asset = reinit_prep(&mut sandbox).await?;

        // rewind!
        timeout(BLOCK_TICK, sandbox.state.confirm_local_height(3)).await??;

        {
            let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;
            assert_eq!(guard.kura().blocks_count(), 3);

            let view = guard.view();
            let value = view.world().assets().get(&asset).unwrap();
            assert_eq!(value.value, Numeric::from(10u32));
        }

        Ok(())
    }

    #[tokio::test]
    async fn soft_fork() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let mut sandbox = Sandbox::new(store.path());

        let block = sandbox.create_block::<InstructionBox>([
            Register::domain(Domain::new(ALICE.domain().to_owned())).into(),
            Register::account(Account::new(ALICE.to_owned())).into(),
        ]);
        timeout(BLOCK_TICK, sandbox.state.insert_block(block)).await??;

        // Create multiple blocks
        for i in 1..=3 {
            let block = sandbox.create_block([Register::asset_definition(
                AssetDefinition::numeric(parse!(format!("rose_{i}#wonderland"))),
            )]);
            timeout(BLOCK_TICK, sandbox.state.insert_block(block)).await??;
        }

        // Verify assets
        {
            let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;
            let assets: Vec<_> = guard
                .view()
                .world()
                .asset_definitions_iter()
                .map(|x| x.id().to_string())
                .collect();

            assert_eq!(
                assets,
                [
                    "rose_1#wonderland",
                    "rose_2#wonderland",
                    "rose_3#wonderland",
                ]
            );
        }

        // Replace top block and insert some more
        sandbox.blocks.pop();
        timeout(
            TICK,
            sandbox.state.confirm_local_height(sandbox.blocks.len()),
        )
        .await??;
        for i in 1..=3 {
            let block = sandbox.create_block([Register::asset_definition(
                AssetDefinition::numeric(parse!(format!("time_{i}#wonderland"))),
            )]);
            timeout(BLOCK_TICK, sandbox.state.insert_block(block)).await??;
        }

        // Verify assets
        {
            let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;
            let assets: Vec<_> = guard
                .view()
                .world()
                .asset_definitions_iter()
                .map(|x| x.id().to_string())
                .collect();

            assert_eq!(
                assets,
                [
                    "rose_1#wonderland",
                    "rose_2#wonderland",
                    "time_1#wonderland",
                    "time_2#wonderland",
                    "time_3#wonderland",
                ]
            );
        }

        Ok(())
    }

    /// We would confirm the height as being just one behind, and then immediately
    /// confirm the previous height. This should cause no-op and normal
    /// continuation of insertion.
    ///
    /// Just a corner case to cover to be sure it doesn't fail.
    #[tokio::test]
    async fn soft_fork_confirm_back() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let mut sandbox = Sandbox::new(store.path());

        let block = sandbox.create_block::<InstructionBox>([
            Register::domain(Domain::new(ALICE.domain().to_owned())).into(),
            Register::account(Account::new(ALICE.to_owned())).into(),
        ]);
        timeout(BLOCK_TICK, sandbox.state.insert_block(block)).await??;

        // Create multiple blocks
        for i in 1..=3 {
            let block = sandbox.create_block([Register::asset_definition(
                AssetDefinition::numeric(parse!(format!("rose_{i}#wonderland"))),
            )]);
            timeout(BLOCK_TICK, sandbox.state.insert_block(block)).await??;
        }

        /// Confirm one behind (prepare state to replace top block)...
        timeout(
            TICK,
            sandbox.state.confirm_local_height(sandbox.blocks.len() - 1),
        )
        .await??;
        /// Confirm back (reset state preparation)
        timeout(
            TICK,
            sandbox.state.confirm_local_height(sandbox.blocks.len()),
        )
        .await??;

        /// Continue insertion
        for i in 4..=6 {
            let block = sandbox.create_block([Register::asset_definition(
                AssetDefinition::numeric(parse!(format!("rose_{i}#wonderland"))),
            )]);
            timeout(BLOCK_TICK, sandbox.state.insert_block(block)).await??;
        }

        // Verify assets
        {
            let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;
            let assets: Vec<_> = guard
                .view()
                .world()
                .asset_definitions_iter()
                .map(|x| x.id().to_string())
                .collect();

            assert_eq!(
                assets,
                [
                    "rose_1#wonderland",
                    "rose_2#wonderland",
                    "rose_3#wonderland",
                    "rose_4#wonderland",
                    "rose_5#wonderland",
                    "rose_6#wonderland",
                ]
            );
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn restore_storage_and_confirm() -> eyre::Result<()> {
        todo!()
    }

    #[tokio::test]
    async fn does_not_shutdown_while_guard_exists() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let sandbox = Sandbox::new(store.path());

        let (tx, mut rx) = mpsc::channel(2);
        let complete = tokio::spawn(async move {
            sandbox.supervisor_task.await.unwrap();
            tx.send(()).await.unwrap();
        });

        let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;
        sandbox.signal.send();

        let _ = timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect_err("must time out");

        drop(guard);

        timeout(TICK, async move {
            rx.recv().await.unwrap();
            // must be no errors
            complete.await.unwrap();
        })
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn does_not_reinit_while_guard_exists() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let mut sandbox = Sandbox::new(store.path());

        // otherwise, soft fork kicks in
        const MIN_BLOCKS_TO_CAUSE_REINIT: usize = 2;

        for _ in 0..MIN_BLOCKS_TO_CAUSE_REINIT {
            let block = sandbox.create_block::<InstructionBox>([]);
            timeout(BLOCK_TICK, sandbox.state.insert_block(block)).await??;
        }

        let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;

        let (tx, mut rx) = mpsc::channel(1);
        let confirm_complete = tokio::spawn({
            let state = sandbox.state.clone();
            async move {
                tracing::info!("confirm 0");
                state.confirm_local_height(0).await.unwrap();
                tracing::info!("confirmed");
                tx.send(()).await.unwrap();
            }
        });

        let _ = timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect_err("must time out");

        drop(guard);

        timeout(TICK, async move {
            rx.recv().await.unwrap();
            confirm_complete.await.unwrap();
        })
        .await?;

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn rejects_block_bad_height() -> eyre::Result<()> {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn rejects_block_bad_hash() -> eyre::Result<()> {
        todo!()
    }

    #[tokio::test]
    #[ignore]
    async fn rejects_block_before_confirm_with_storage() -> eyre::Result<()> {
        todo!()
    }

    #[tokio::test]
    async fn sends_metrics() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let mut sandbox = Sandbox::new(store.path());

        let mut rx = sandbox.telemetry.take().unwrap();
        let mut recv = async || {
            let Some(telemetry::ActorMessage::UpdateBlockchainState(data)) =
                timeout(TICK, rx.recv())
                    .await
                    .expect("must receive metrics within timeout")
            else {
                unreachable!()
            };
            data
        };

        let metrics = recv().await;
        assert_eq!(metrics.block, 0);

        let block = sandbox.create_block::<InstructionBox>([
            Register::domain(Domain::new(ALICE.domain().to_owned())).into(),
            Register::account(Account::new(ALICE.to_owned())).into(),
        ]);
        timeout(TICK, sandbox.state.insert_block(block)).await??;

        let metrics = recv().await;
        assert_eq!(metrics.block, 1);
        assert_eq!(metrics.accounts, 2);
        assert_eq!(metrics.domains, 2);

        // TODO: sends on reinit/rewind; reflects different entities properly

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn telemetry_being_full_does_not_hang_state() -> eyre::Result<()> {
        todo!()
    }

    #[tokio::test]
    async fn block_errors_are_written() -> eyre::Result<()> {
        init_test_logger();

        let store = tempfile::tempdir()?;
        let mut sandbox = Sandbox::new(store.path());

        let block = sandbox.create_block([
            // non-existent asset
            Mint::asset_numeric(3u32, parse!(format!("time##{}", *ALICE))),
        ]);
        timeout(TICK, sandbox.state.insert_block(block)).await??;

        let guard = timeout(TICK, sandbox.state.acquire_guard()).await?;
        let block = guard.kura().get_block(nonzero!(1usize)).unwrap();

        assert_eq!(block.errors().len(), 1);

        Ok(())
    }
}
