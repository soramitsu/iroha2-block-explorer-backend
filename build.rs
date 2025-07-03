use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize)]
struct Metadata {
    #[serde(rename(serialize = "versions"))]
    #[serde(deserialize_with = "deserialize_data")]
    packages: Vec<Data>,
}

#[derive(Serialize, Deserialize)]
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

    let cargo_metadata = std::str::from_utf8(&output.stdout).unwrap();
    let version_metadata = serde_json::from_str::<Metadata>(cargo_metadata).unwrap();
    let version_metadata = serde_json::to_string(&version_metadata).unwrap(); 

    println!("cargo:rustc-env=VERSION_METADATA={version_metadata}");

    // panic!("{}", version_metadata);
}
