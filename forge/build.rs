fn main() {
    // Cargo semver doesn't allow leading zeros (e.g. 2026.04.04 is invalid),
    // but we want calver with leading zeros in the CLI output (YYYY.MM.DD).
    // Re-format CARGO_PKG_VERSION to zero-pad month and day components.
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() == 3 {
        let formatted = format!(
            "{}.{:0>2}.{:0>2}",
            parts[0], parts[1], parts[2]
        );
        println!("cargo:rustc-env=FORGE_VERSION={formatted}");
    } else {
        println!("cargo:rustc-env=FORGE_VERSION={version}");
    }
}
