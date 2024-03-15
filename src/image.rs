//! Registry operations <https://specs.opencontainers.org/distribution-spec/?v=v1.0.0>

use std::{collections::HashMap, path::PathBuf};

use bytes::Bytes;
use eyre::{bail, Context, ContextCompat, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    RequestBuilder, Response, Url,
};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::util;

struct BaseUrl(Url);

impl BaseUrl {
    fn with_path(&self, p: impl AsRef<str>) -> Url {
        let mut u = self.0.clone();
        u.set_path(&(self.0.path().to_owned() + p.as_ref()));
        u
    }
}

pub struct Client {
    c: reqwest::Client,
    base: BaseUrl,
    token: String,
}

#[derive(Deserialize)]
struct AuthTokenResponse {
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciImageIndex {
    pub schema_version: monostate::MustBe!(2_u64),
    pub manifests: Vec<OciImageIndexManifest>,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciImageIndexManifest {
    pub media_type: monostate::MustBe!("application/vnd.oci.image.manifest.v1+json"),
    pub platform: OciImageIndexManifestPlatform,
    pub digest: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciImageIndexManifestPlatform {
    pub architecture: String,
    pub os: String,
    #[serde(rename = "os.version")]
    pub os_version: Option<String>,
    #[serde(rename = "os.features")]
    #[serde(default)]
    pub os_features: Vec<String>,
    pub variant: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciImageIndexManifestEntry {
    pub schema_version: monostate::MustBe!(2),
    pub media_type: monostate::MustBe!("application/vnd.oci.image.manifest.v1+json"),
    pub config: OciImageConfigRef,
    pub layers: Vec<OciImageLayer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciImageConfigRef {
    pub digest: String,
    pub media_type: monostate::MustBe!("application/vnd.oci.image.config.v1+json"),
    pub size: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciImageLayer {
    pub digest: String,
    pub media_type: monostate::MustBe!("application/vnd.oci.image.layer.v1.tar+gzip"),
    pub size: u64,
}

impl Client {
    pub fn new(base: Url) -> Self {
        let mut c = reqwest::ClientBuilder::new();

        c = c.user_agent("athlete (https://github.com/Nilstrieb/athlete)");
        c = c.danger_accept_invalid_certs(true);
        let mut headers = HeaderMap::new();
        headers.append(
            "Docker-Distribution-Api-Version",
            HeaderValue::from_static("registry/2.0"),
        );
        c = c.default_headers(headers);

        Self {
            c: c.build().unwrap(),
            base: BaseUrl(base),
            token: String::new(),
        }
    }

    pub async fn token(&mut self, scope: &str) -> Result<()> {
        let mut url = Url::parse("https://auth.docker.io/token").unwrap();
        url.query_pairs_mut()
            .append_pair("scope", scope)
            .append_pair("service", "registry.docker.io")
            .finish();

        let token = self
            .c
            .get(url)
            .send()
            .await
            .wrap_err("sending login request")?
            .json::<AuthTokenResponse>()
            .await
            .wrap_err("fetching login token")?;

        self.token = format!("Bearer {}", token.token);
        Ok(())
    }

    pub async fn get_manifests(&self, image: &str, reference: &str) -> Result<OciImageIndex> {
        make_request(
            self.c
                .get(
                    self.base
                        .with_path(format!("{image}/manifests/{reference}")),
                )
                .header("Accept", "application/vnd.oci.image.index.v1+json")
                .header("Authorization", &self.token),
        )
        .await?
        .json::<OciImageIndex>()
        .await
        .wrap_err("fetching manifest body")
    }

    pub async fn get_manifest_from_digest(&self, image: &str, digest: &str) -> Result<Bytes> {
        make_request(
            self.c
                .get(self.base.with_path(format!("{image}/manifests/{digest}")))
                .header("Accept", "application/vnd.oci.image.manifest.v1+json")
                .header("Authorization", &self.token),
        )
        .await?
        .bytes()
        .await
        .wrap_err("fetching manifest body")
    }
    pub async fn get_blob(&self, image: &str, digest: &str) -> Result<bytes::Bytes> {
        make_request(
            self.c
                .get(self.base.with_path(format!("{image}/blobs/{digest}")))
                .header("Authorization", &self.token),
        )
        .await?
        .bytes()
        .await
        .wrap_err("fetching blob body")
    }

    pub async fn pull(&self, image: &str, reference: &str) -> Result<()> {
        let index = self
            .get_manifests(image, reference)
            .await
            .wrap_err("fetching index")?;

        tracing::debug!("Fetched {} manifests", index.manifests.len());

        let Some(version) = index.manifests.iter().find(|manifest| {
            manifest.platform.architecture == crate::util::host_oci_arch()
                && manifest.platform.os == crate::util::host_oci_os()
        }) else {
            bail!("no image with matching architecture and operating system found");
        };

        tracing::debug!(?version, "Found matching manifest {}", version.digest);

        let manifest_bytes = self
            .get_manifest_from_digest(image, &version.digest)
            .await?;
        let manifest = serde_json::from_slice::<OciImageIndexManifestEntry>(&manifest_bytes)
            .wrap_err("invalid manifest")?;

        tracing::debug!(?manifest, "Fetched manifest entry");

        let config_blob = self
            .get_blob(image, &manifest.config.digest)
            .await
            .wrap_err("fetching blob or something")?;

        let config = serde_json::from_slice::<OciImageConfig>(&config_blob)
            .wrap_err("parsing image config")?;

        tracing::debug!(created = ?config.created, "Fetched configuration");

        // FIXME: handle multiple tags or something like that
        let writer = ImageLayoutWriter::init(format!(".cache/images/{image}").into(), &index)
            .await
            .wrap_err("creating image storage")?;
        writer.write_blob(&version.digest, &manifest_bytes).await?;
        writer
            .write_blob(&manifest.config.digest, &config_blob)
            .await?;

        Ok(())
    }
}

pub async fn make_request(req: RequestBuilder) -> Result<Response> {
    req.send()
        .await
        .wrap_err("making request")?
        .error_for_status()
        .wrap_err("http error in request")
}

#[derive(Debug, Deserialize)]
pub struct OciImageConfig {
    pub created: Option<String>,
    pub author: Option<String>,
    pub architecture: String,
    pub os: String,
    pub config: Option<OciImageConfigConfig>,
    pub rootfs: OciImageConfigRootfs,
    pub history: Option<Vec<OciImageConfigHistory>>,
}

#[derive(Debug, Deserialize)]
pub struct OciImageConfigRootfs {
    pub diff_ids: Vec<String>,
    pub r#type: String,
}

#[derive(Debug, Deserialize)]
pub struct OciImageConfigHistory {
    pub created: Option<String>,
    pub created_by: Option<String>,
    pub empty_layer: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OciImageConfigConfig {
    pub user: Option<String>,
    pub exposed_ports: Option<HashMap<String, serde_json::Value>>,
    pub env: Option<Vec<String>>,
    pub entrypoint: Option<Vec<String>>,
    pub cmd: Vec<String>,
    pub volumes: Option<HashMap<String, serde_json::Value>>,
    pub working_dir: Option<String>,
    pub labels: Option<HashMap<String, String>>,
    pub stop_signal: Option<String>,
    // this exists but isn't specified:
    // attach_stderr: bool,
    // attach_stdin: bool,
    // attach_stdout: bool,
    // domainname: String,
    // hostname: String,
    // image: String,
    // on_build: (),
    // open_stdin: bool,
    // stdin_once: bool,
    // tty: bool,
}

pub struct ImageLayoutWriter {
    // look, this could be a `Dir`` but that's too annoying right now
    dir: PathBuf,
}

impl ImageLayoutWriter {
    pub async fn init(dir: PathBuf, index: &OciImageIndex) -> Result<Self> {
        fs::create_dir_all(&dir)
            .await
            .wrap_err_with(|| format!("creating {}", dir.display()))?;

        util::write_file(
            dir.join("oci-layout"),
            r#"{
            "imageLayoutVersion": "1.0.0"
        }
"#,
        )
        .await?;
        let index = serde_json::to_vec(index).wrap_err("serializing index")?;
        util::write_file(dir.join("index.json"), &index).await?;

        Ok(Self { dir })
    }

    pub async fn write_blob(&self, digest: &str, blob_content: &[u8]) -> Result<()> {
        let (alg, encoded) = digest
            .split_once(":")
            .wrap_err_with(|| format!("digest {digest} does not have ALG:ENCODED format"))?;

        let blobs = self.dir.join("blobs").join(alg);
        fs::create_dir_all(&blobs)
            .await
            .wrap_err_with(|| format!("creating {}", blobs.display()))?;

        util::write_file(blobs.join(encoded), blob_content).await?;

        Ok(())
    }
}
