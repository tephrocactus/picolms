mod health;
mod state;

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
pub use state::State;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

pub async fn start_server(
    addr: SocketAddr,
    tls: RustlsConfig,
    state: State,
    ct: CancellationToken,
) -> Result<()> {
    let listener = TcpListener::bind(addr).rustls(tls);
    let router = Route::new().at("/", get(health::handler)).data(state);
    Server::new(listener)
        .run_with_graceful_shutdown(router, ct.cancelled_owned(), None)
        .await
        .context("run")
}

pub fn tls_config(ca: &PathBuf, crt: &PathBuf, key: &PathBuf) -> Result<RustlsConfig> {
    let ca = std::fs::read(ca).with_context(|| ca.display().to_string())?;
    let crt = std::fs::read(crt).with_context(|| crt.display().to_string())?;
    let key = std::fs::read(key).with_context(|| key.display().to_string())?;
    Ok(RustlsConfig::new()
        .client_auth_required(ca)
        .fallback(RustlsCertificate::new().key(key).cert(crt)))
}
