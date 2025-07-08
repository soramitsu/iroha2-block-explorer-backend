pub mod blockchain;

use serde::Serialize;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

pub trait ExpectExt {
    fn assert_json_eq(&self, actual: impl Serialize);
}

impl ExpectExt for expect_test::Expect {
    fn assert_json_eq(&self, actual: impl Serialize) {
        let json = serde_json::to_string_pretty(&actual).unwrap();
        self.assert_eq(&json);
    }
}

pub fn init_test_logger() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "debug,iroha_explorer_core=trace,iroha_explorer_telemetry=trace,iroha_core=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
}
