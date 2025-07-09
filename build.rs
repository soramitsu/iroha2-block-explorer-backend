use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
struct Metadata {
    #[serde(rename(deserialize = "packages"))]
    #[serde(deserialize_with = "deserialize_data")]
    versions: Vec<Data>,
}

#[derive(Deserialize)]
struct Data {
    name: String,
    version: String,
}

fn deserialize_data<'de, D>(deserializer: D) -> Result<Vec<Data>, D::Error>
where
    D: Deserializer<'de>,
{
    let metadata = Vec::<Data>::deserialize(deserializer)?;
    Ok(metadata
        .into_iter()
        .filter(|data| matches!(data.name.as_str(), "iroha_data_model" | "iroha_explorer"))
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

    let cargo_metadata = std::str::from_utf8(&output.stdout).unwrap();
    let metadata = serde_json::from_str::<Metadata>(cargo_metadata).unwrap();

    for Data { version, name } in &metadata.versions {
        if name == "iroha_explorer" {
            println!("cargo:rustc-env=VERGEN_EXPLORER_VERSION={version}");
        } else if name == "iroha_data_model" {
            println!("cargo:rustc-env=VERGEN_IROHA_COMPAT=v{version}");
        }
    }
}
