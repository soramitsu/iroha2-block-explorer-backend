use serde::Deserialize;
use std::str::FromStr;

#[derive(Deserialize)]
struct Data {
    name: String,
    version: String,
}

enum ExplorerMetadata {
    ExplorerVersion,
    IrohaCompatibility,
    Other,
}

impl FromStr for ExplorerMetadata {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "iroha_explorer" => Ok(Self::ExplorerVersion),
            "iroha_data_model" => Ok(Self::IrohaCompatibility),
            _ => Ok(Self::Other),
        }
    }
}

fn main() {
    vergen::EmitBuilder::builder()
        .git_sha(true)
        .cargo_features()
        .emit()
        .unwrap();

    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .output()
        .unwrap();

    let metadata = std::str::from_utf8(&output.stdout).unwrap();
    let metadata = serde_json::from_str::<serde_json::Value>(metadata).unwrap();
    let versions = serde_json::from_value::<Vec<Data>>(metadata["packages"].clone()).unwrap();

    for Data { version, name } in &versions {
        match name.parse::<ExplorerMetadata>() {
            Ok(ExplorerMetadata::ExplorerVersion) => {
                println!("cargo:rustc-env=VERGEN_EXPLORER_VERSION={version}")
            }
            Ok(ExplorerMetadata::IrohaCompatibility) => {
                println!("cargo:rustc-env=VERGEN_IROHA_COMPAT=v{version}")
            }
            _ => {
                // ignore everything else...
            }
        }
    }
}
