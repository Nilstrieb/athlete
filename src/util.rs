use std::path::Path;

use eyre::{bail, Context};
use sha2::Digest;

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

pub async fn digest(alg: &str, content: Vec<u8>) -> eyre::Result<String> {
    let alg = alg.to_owned();
    tokio::task::spawn_blocking(move || match alg.as_str() {
        "sha256" => {
            let mut hasher = sha2::Sha256::new();
            hasher.update(content);
            Ok(hex::encode(hasher.finalize()))
        }
        "sha512" => {
            let mut hasher = sha2::Sha512::new();
            hasher.update(content);
            Ok(hex::encode(hasher.finalize()))
        }
        _ => bail!("unrecognized hashing algorithm '{alg}'"),
    })
    .await
    .wrap_err("failed to spawn blocking task")?
}
