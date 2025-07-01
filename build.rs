fn main() {
    let output = std::process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .expect("build script is executed with `git` installed and `.git` available");

    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=VERGEN_GIT_SHA={}", git_hash.trim());

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
}
