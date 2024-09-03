use anyhow::Context;
use anyhow::Result;
use poem::get;
use poem::listener::Listener;
use poem::listener::RustlsCertificate;
use poem::listener::RustlsConfig;
use poem::listener::TcpListener;
use poem::EndpointExt;
use poem::Route;
use poem::Server;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

mod health;
mod state;

pub use state::State;

pub struct Config {
    pub addr: SocketAddr,
    pub ca: PathBuf,
    pub crt: PathBuf,
    pub key: PathBuf,
}

pub async fn start_server(cfg: Config, state: State, ct: CancellationToken) -> Result<()> {
    let listener = TcpListener::bind(cfg.addr).rustls({
        let ca = std::fs::read(&cfg.ca).with_context(|| format!("read {}", cfg.ca.display()))?;
        let crt = std::fs::read(&cfg.crt).with_context(|| format!("read {}", cfg.crt.display()))?;
        let key = std::fs::read(&cfg.key).with_context(|| format!("read {}", cfg.key.display()))?;
        RustlsConfig::new()
            .client_auth_required(ca)
            .fallback(RustlsCertificate::new().key(key).cert(crt))
    });

    let router = Route::new().at("/", get(health::handler)).data(state);

    Server::new(listener)
        .run_with_graceful_shutdown(router, ct.cancelled_owned(), None)
        .await
        .context("run")
}
