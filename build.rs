use std::str::FromStr;

use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
struct VersionMetadata {
    #[serde(rename(deserialize = "packages"))]
    #[serde(deserialize_with = "deserialize_data")]
    versions: Vec<Data>,
}

#[derive(Deserialize)]
struct Data {
    name: String,
    version: String,
}

#[derive(PartialEq, Debug)]
enum ExplorerMetadata {
    ExplorerVersion,
    IrohaCompatibility,
    Unknown,
}

impl FromStr for ExplorerMetadata {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "iroha_explorer" => Ok(Self::ExplorerVersion),
            "iroha_data_model" => Ok(Self::IrohaCompatibility),
            _ => Ok(Self::Unknown),
        }
    }
}

fn deserialize_data<'de, D>(deserializer: D) -> Result<Vec<Data>, D::Error>
where
    D: Deserializer<'de>,
{
    let metadata = Vec::<Data>::deserialize(deserializer)?;
    Ok(metadata
        .into_iter()
        .filter(|data| data.name.starts_with("iroha"))
        .collect::<Vec<_>>())
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
    let metadata = serde_json::from_str::<VersionMetadata>(metadata).unwrap();

    for Data { version, name } in &metadata.versions {
        match name.parse::<ExplorerMetadata>() {
            Ok(ExplorerMetadata::ExplorerVersion) => {
                println!("cargo:rustc-env=VERGEN_EXPLORER_VERSION={version}")
            }
            Ok(ExplorerMetadata::IrohaCompatibility) => {
                println!("cargo:rustc-env=VERGEN_IROHA_COMPAT=v{version}")
            }
            Ok(ExplorerMetadata::Unknown) => {
                // skip everthing else
            }
            Err(_) => unreachable!(),
        }
    }
}
