fn main() {
    // Allow overriding version via BUILD_VERSION env var at build time.
    // Usage: BUILD_VERSION="0.2.0-beta3" cargo build
    // Falls back to the Cargo.toml package version if not set.
    let version = std::env::var("BUILD_VERSION")
        .unwrap_or_else(|_| std::env::var("CARGO_PKG_VERSION").unwrap());
    println!("cargo:rustc-env=BUILD_VERSION={version}");
    // Re-run only if the env var changes.
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");
}
