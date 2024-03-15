use std::path::PathBuf;

use clap::Parser;
use eyre::Context;
use reqwest::Url;

use athlete::{image, runtime};
use tracing_subscriber::EnvFilter;

#[derive(clap::Parser)]
#[command(name = "athlete")]
struct Opts {
    #[command(subcommand)]
    command: Subcommand,
}

#[derive(clap::Subcommand)]
enum Subcommand {
    /// Runtime commands
    Rt {
        #[command(subcommand)]
        cmd: RtCommand,
    },
    /// Registry-related commands
    Registry {
        #[command(subcommand)]
        cmd: RegistryCommand,
    },
}

#[derive(clap::Subcommand)]
enum RtCommand {
    State {
        container_id: runtime::ContainerId,
    },
    Create {
        container_id: runtime::ContainerId,
        bundle_path: PathBuf,
    },
    Start {
        container_id: runtime::ContainerId,
    },
    Kill {
        container_id: runtime::ContainerId,
        signal: u8,
    },
    Delete {
        container_id: runtime::ContainerId,
    },
}

#[derive(clap::Subcommand)]
enum RegistryCommand {
    Pull { image: String, reference: String },
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let opts = Opts::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::builder().parse_lossy(
            std::env::var("ATHLETE_LOG").unwrap_or_else(|_| "athlete=debug,info".into()),
        ))
        .init();

    match opts.command {
        Subcommand::Rt { cmd } => {
            let runtime = runtime::Runtime {};
            match cmd {
                RtCommand::State { container_id } => {
                    let state = runtime.state(container_id)?;
                    println!("{:?}", state);
                }
                RtCommand::Create {
                    container_id,
                    bundle_path,
                } => runtime.create(container_id, &bundle_path)?,
                RtCommand::Start { container_id } => runtime.start(container_id)?,
                RtCommand::Kill {
                    container_id,
                    signal,
                } => runtime.kill(container_id, signal)?,
                RtCommand::Delete { container_id } => runtime.delete(container_id)?,
            }
        }
        Subcommand::Registry { cmd } => match cmd {
            RegistryCommand::Pull { image, reference } => {
                let mut client = image::Client::new(
                    Url::parse("https://registry-1.docker.io/v2/library/").unwrap(),
                );
                client
                    .token(&format!("repository:library/{image}:pull"))
                    .await
                    .wrap_err("logging in")?;
                tracing::debug!("Login succeeded");
                client
                    .pull(&image, &reference)
                    .await
                    .wrap_err_with(|| format!("pullling {image}:{reference}"))?;
            }
        },
    }

    Ok(())
}
