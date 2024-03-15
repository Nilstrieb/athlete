use std::path::Path;

use eyre::Context;

pub fn host_oci_arch() -> &'static str {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "x86_64")] {
            "amd64"
        } else if #[cfg(target_arch = "aarch64")] {
            "arm64"
        } else {
            compile_error!("unsupported architecture")
        }
    }
}

pub fn host_oci_os() -> &'static str {
    cfg_if::cfg_if! {
        if #[cfg(target_os = "linux")] {
            "linux"
        } else {
            compile_error!("unsupported architecture")
        }
    }
}

pub async fn write_file(path: impl AsRef<Path>, content: impl AsRef<[u8]>) -> eyre::Result<()> {
    let path = path.as_ref();
    tokio::fs::write(path, content)
        .await
        .wrap_err_with(|| format!("writing to {}", path.display()))
}
