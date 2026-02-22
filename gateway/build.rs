use anyhow::Result;

fn main() -> Result<()> {
    let app_version = env("CARGO_PKG_VERSION")?;
    let app_version = match app_version.to_string_lossy() {
        std::borrow::Cow::Borrowed(version) => version,
        std::borrow::Cow::Owned(version) => {
            anyhow::bail!("invalid CARGO_PKG_VERSION: {version}")
        }
    };

    let rustc_version = rustc_version::version()?;

    println!("cargo:rustc-env=GATEWAY_VERSION={app_version}");
    println!("cargo:rustc-env=GATEWAY_RUSTC_VERSION={rustc_version}");
    Ok(())
}

fn env(key: &str) -> Result<std::ffi::OsString> {
    println!("cargo:rerun-if-env-changed={key}");
    std::env::var_os(key).ok_or_else(|| anyhow::anyhow!("missing '{key}' environment variable"))
}
