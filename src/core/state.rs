use std::{future::Future, num::NonZero, path::PathBuf, sync::Arc};

use crate::core::StateViewExt;

use super::storage::{self, ReadGuard, Storage};
use super::{Error, Result};
use iroha_core::block::{CommittedBlock, ValidBlock};
use iroha_core::kura::{self, Kura};
use iroha_core::state::{State as CoreState, StateBlock, StateView};
use iroha_data_model::prelude::*;
use iroha_futures::supervisor::ShutdownSignal;
use tokio::sync::{mpsc, oneshot, watch};

type BlockHash = HashOf<BlockHeader>;

#[derive(Clone)]
pub struct State {
    actor: mpsc::Sender<Message>,
}

struct Actor {
    handle: mpsc::Receiver<Message>,
    storage: Storage,
    state: Option<Arc<CoreState>>,
}

enum Message {
    Height {
        reply: oneshot::Sender<usize>,
    },
    State {
        reply: oneshot::Sender<Option<StateReader>>,
    },
    Insert {
        block: Arc<SignedBlock>,
        reply: oneshot::Sender<Result<()>>,
    },
}

/// A reader of the state, whose purpose is to ensure
/// that storage won't be terminated while it exists.
#[derive(Clone)]
pub struct StateReader {
    core: Arc<CoreState>,
    read_guard: ReadGuard,
}

impl StateReader {
    /// Get a persistent state view.
    ///
    /// For consistency, it is important to use a single [`StateView`].
    /// Subsequent calls may result in different views.
    pub fn view(&self) -> StateView<'_> {
        self.core.view()
    }

    pub fn storage(&self) -> &ReadGuard {
        &self.read_guard
    }
}

impl State {
    pub fn new(storage: Storage) -> (Self, impl Future<Output = ()> + Sized) {
        todo!()
    }

    // TODO: use some ActorCommunicationError
    pub async fn height(&self) -> Result<usize> {
        let (tx, rx) = oneshot::channel();
        self.actor.send(Message::Height { reply: tx }).await?;
        let reply = rx.await?;
        Ok(reply)
    }

    pub async fn height_watch(&self) -> Result<watch::Receiver<usize>> {
        todo!()
    }

    // pub async fn block_hash(&self, height: NonZero<usize>) -> eyre::Result<Option<BlockHash>> {
    //     todo!()
    // }
    //

    /// Tell which latest block height in the local storage is "confirmed" to match with
    /// remote block chain.
    ///
    /// This may cause state and storage re-initialisation.
    ///
    /// TODO: blocks parallel acquire_read? resolves them?
    ///
    /// As a result, the state will reflect the given height.
    pub async fn confirm_local_height(&self, height: usize) -> Result<()> {
        todo!()
    }

    /// Acquire a state reader.
    ///
    /// [None] if the state is not ready yet.
    pub async fn acquire_read(&self) -> Result<Option<StateReader>> {
        let (tx, rx) = oneshot::channel();
        self.actor.send(Message::State { reply: tx }).await?;
        let reply = rx.await?;
        Ok(reply)
    }

    /// Insert a block into the state.
    ///
    /// TODO: makes block validation in parallel? Allows parallel acquire_read (by not blocking the
    /// actors loop?)
    pub async fn insert_block(&self, block: Arc<SignedBlock>) -> Result<Result<()>> {
        let (tx, rx) = oneshot::channel();
        self.actor
            .send(Message::Insert { block, reply: tx })
            .await?;
        rx.await?
    }
}

impl Actor {
    async fn recv_loop(&mut self) {
        while let Some(msg) = self.handle.recv().await {
            match msg {
                Message::State { reply } => {
                    let guard = self.storage.acquire_read().await;
                    let _ = reply.send(Some((self.state.clone(), guard)));
                }
                Message::Height { reply } => {
                    todo!()
                }
                Message::Insert { block } => {
                    todo!()
                }
            }
        }
    }

    /// Try to insert a block received from Iroha (the source of truth) to the local block chain.
    ///
    /// Requirements:
    ///
    /// * The received block must _not_ already be in the local block chain.
    /// * The received block height must _not_ be greater than the local height by more than 1.
    /// * The received block's previous block hash must be in the local state.
    ///
    /// If the received block is not in the block chain, but its previous block _is_, then the received
    /// block is inserted. If there are irrelevant blocks (i.e. with height greater or equal to the
    /// received one), they are discarded.
    ///
    /// Success makes the received block be the latest block in the local block chain.
    /// The world state is updated accordingly.
    fn try_insert_block(&self, block: SignedBlock) -> Result<()> {
        let view = self.state.view();

        let state_height = view.height();
        let block_height: NonZero<usize> = block
            .header()
            .height()
            .try_into()
            .expect("must fit into usize");

        if block_height.get() > state_height + 1 {
            return Err(Error::ReceivedBlockHeightIsTooFar {
                state_height,
                block_height,
            });
        }

        let block_hash = block.hash();
        if let Some(local_same_height_block_hash) = view.block_hash(block_height) {
            if block_hash == local_same_height_block_hash {
                return Err(Error::ReceivedBlockIsAlreadyInBlockChain);
            }
        }

        if block_height.get() == 1 {
            // Genesis - re-create the state
            self.reinit(infer_genesis_account(&block)?)?;
            self.do_insert_block(block);
        } else {
            let block_prev_block_hash = block
                .header()
                .prev_block_hash()
                .expect("only None for genesis");
            let prev_block_height =
                NonZero::new(block_height.get().checked_sub(1).expect("non-zero"))
                    .expect("not genesis");
            let local_prev_block_hash = view.block_hash(prev_block_height)
                .expect("must be: received block is not genesis and local height is at most one block behind");

            if local_prev_block_hash != block_prev_block_hash {
                return Err(Error::ReceivedBlockPreviousBlockHashNotFound {
                    actual_prev_block_hash: local_prev_block_hash,
                    block_prev_block_hash,
                });
            }

            // If local height is behind by one, just apply
            // If local height is the same, revert and apply
            // If local height is ahead, then re-create the state by re-applying _all_ blocks
            // behind first, then applying the received one

            if block_height.get() == state_height + 1 {
                self.do_insert_block(block);
            } else if block_height.get() == state_height {
                self.do_replace_top_block(block);
            } else {
                assert!(state_height > block_height.get());

                // 1. Drop current state, delete from sup
                // 2. Create new state with genesis from kura's genesis
                // 3. Play blocks from kura until block_height - 1
                // 4. Discard other blocks in Kura
                // 5. Insert new block normally
                //
                // Requirements:
                //
                // 1. Add `Kura::discard_from(height: NonZero<usize>)`
                // 2. Tell supervisor to shutdown a child normally (live query store?)
                //
                // Or:
                //
                // 1. Drop current state & kura, stop with supervisor
                // 2. Create new state, StorageInit::Restore
                // 2a. Tell storage to discard blocks from a certain height
                // 3. Play all blocks from storage
                // 4. Insert new block
                //
                // Pros: no need to tweak the supervisor
                // Pros: can discard Kura blocks on init, easier to implement
                //

                todo!()
            }
            // self.h
        }

        todo!()
    }

    fn reinit(&mut self, genesis_account: &AccountId) -> Result<()> {
        // TODO: abort previous supervisor & kura

        let mut sup = Supervisor::new();
        let storage = Storage::new(&self.store_dir, &mut sup, StorageInit::New)?;
        let state = init_state(&mut sup, storage.kura().clone(), genesis_account)?;

        let shutdown = sup.shutdown_signal();
        let sup_fut = sup.start();

        self.storage = storage;
        self.state = Arc::new(state);
        self.sup = Arc::new(sup);

        Ok(())
    }

    fn do_insert_block(&self, block: SignedBlock) {
        let state_block = self.state.block(block.header());
        let committed_block = play_block(block, state_block);
        self.storage.kura().store_block(committed_block);
    }

    fn do_replace_top_block(&self, block: SignedBlock) {
        let state_block = self.state.block_and_revert(block.header());
        let committed_block = play_block(block, state_block);
        self.storage.kura().replace_top_block(committed_block);
    }
}

// fn create_state(kura: &Kura) ->

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

fn init_state(sup: &mut Supervisor, kura: Arc<Kura>, genesis_account: &AccountId) -> Result<State> {
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

    let state = State::new(world, kura, query_handle);

    Ok(state)
}
