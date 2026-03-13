use anyhow::Result;
use clap::Parser;
use tonic::transport::Server;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod service;

pub mod proto {
    tonic::include_proto!("pwright.v1");
}

/// pwright — Lightweight Rust CDP bridge with gRPC
#[derive(Parser, Debug)]
#[command(name = "pwright", version, about)]
struct Cli {
    /// Chrome DevTools Protocol WebSocket URL
    #[arg(long, env = "CDP_URL")]
    cdp_url: Option<String>,

    /// gRPC server listen address
    #[arg(long, default_value = "127.0.0.1:50051", env = "GRPC_ADDR")]
    addr: String,

    /// Bind to all network interfaces (0.0.0.0) instead of localhost only
    #[arg(long, short = 'B')]
    bind_all: bool,

    /// Disable the Evaluate RPC (blocks arbitrary JavaScript execution)
    #[arg(long, env = "PWRIGHT_DISABLE_EVAL")]
    disable_eval: bool,

    /// Maximum parallel tab operations
    #[arg(long, default_value = "4", env = "MAX_PARALLEL_TABS")]
    max_parallel_tabs: usize,

    /// Default navigation timeout in milliseconds
    #[arg(long, default_value = "30000", env = "NAV_TIMEOUT_MS")]
    nav_timeout_ms: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let addr_str = if cli.bind_all {
        let port = cli.addr.rsplit_once(':').map(|(_, p)| p).unwrap_or("50051");
        format!("0.0.0.0:{}", port)
    } else {
        cli.addr.clone()
    };

    info!(
        addr = addr_str,
        cdp_url = cli.cdp_url.as_deref().unwrap_or("<not set>"),
        disable_eval = cli.disable_eval,
        "starting pwright gRPC server"
    );

    if cli.bind_all {
        warn!(
            "--bind-all is set: gRPC server is accessible from the network. Ensure proper firewall rules are in place."
        );
    }

    let service = service::BrowserServiceImpl::new(
        cli.cdp_url,
        cli.max_parallel_tabs,
        cli.nav_timeout_ms,
        cli.disable_eval,
    );

    let addr = addr_str.parse()?;
    info!(%addr, "listening for gRPC connections");

    Server::builder()
        .add_service(proto::browser_service_server::BrowserServiceServer::new(
            service,
        ))
        .serve(addr)
        .await?;

    Ok(())
}
