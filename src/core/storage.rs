use iroha_core::kura::{Kura, KuraReadOnly};
use iroha_data_model::prelude::*;
use iroha_futures::supervisor::ShutdownSignal;
use std::{future::Future, num::NonZero, ops::Deref, path::PathBuf, sync::Arc};
use tokio::sync::mpsc;

use super::Result;

#[derive(Clone)]
pub struct Storage {
    actor: mpsc::Sender<()>,
}

pub struct ReadGuard();

pub struct WriteGuard();

pub enum StartMode {
    /// Wipe the storage
    Wipe,
    /// Restore existing storage, but wipe blocks after the given height
    Restore { height: NonZero<usize> },
}

impl Storage {
    pub fn new(
        store_dir: impl Into<PathBuf>,
        signal: ShutdownSignal,
    ) -> (Self, impl Future<Output = ()> + Sized) {
        todo!()
    }

    /// Acquire read access, allowing to read while being sure
    /// blocks aren't added/replaced in Kura.
    ///
    /// Waits until all existing [`WriteGuard`]s are dropped.
    ///
    /// Prevents storage termination until the guard is dropped.
    ///
    /// Error if called during restart.
    /// This conflict should not occur in Explorer: restart happens in State loop
    /// (thus, no new query reads could be acquired) on block insertion, which could not
    /// overlap with `acquire_read` in sync.
    ///
    /// TODO: or just wait for restart?
    pub async fn acquire_read(&self) -> Result<ReadGuard> {
        todo!()
    }

    /// Acquire write access, allowing to add/replace blocks in Kura.
    ///
    /// Waits until all existing [`ReadGuard`]s are dropped.
    /// Puts any new `acquire_read` on hold until all `acquire_write`s are satisfied.
    ///
    /// Prevents storage termination until the guard is dropped.
    ///
    /// Error if called during restart.
    /// This conflict should not occur in Explorer: write is only acquired by State
    /// and only after it provokes the restart by itself.
    /// This just shouldn't happen logically in Explorer
    /// and this case is not covered intentionally.
    pub async fn acquire_write(&self) -> Result<WriteGuard> {
        todo!()
    }

    /// Restart the storage.
    ///
    /// Waits until all [`ReadGuard`]s and [`WriteGuard`]s are dropped,
    /// then gracefully shut downs Kura and starts from scratch.
    ///
    /// Resolves once the entire process finishes.
    pub async fn restart(&self, mode: StartMode) -> Result<()> {}
}

impl ReadGuard {
    pub fn height(&self) -> usize {}

    pub fn block_hash(&self, height: NonZero<usize>) -> Option<HashOf<BlockHeader>> {}
}

impl KuraReadOnly for ReadGuard {
    fn get_block(&self, height: NonZero<usize>) -> Option<Arc<SignedBlock>> {
        todo!()
    }
}

impl Deref for WriteGuard {
    type Target = Kura;

    fn deref(&self) -> &Self::Target {
        todo!()
    }
}
