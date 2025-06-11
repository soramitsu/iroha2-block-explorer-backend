pub mod query;

use iroha_core::state::State as CoreState;
pub use query::QueryExecutor;

pub struct State {
    core: CoreState,
}

impl State {
    pub fn query(&self) -> QueryExecutor<'_> {
        QueryExecutor::new(self.core.view())
    }
}
