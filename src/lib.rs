mod api;
mod engine;
pub(crate) mod picodata;

use crate::api::tls_config;
use crate::engine::Engine;
use crate::picodata::rpc::ProxyClient;
use crate::picodata::service::ServiceConfig;
use crate::picodata::service::ServiceErrors;
use anyhow::Result;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

pub async fn entrypoint(
    cfg: ServiceConfig,
    rpc_client: ProxyClient,
    tt: TaskTracker,
    ct: CancellationToken,
    se: ServiceErrors,
) -> Result<()> {
    let engine = Engine::new();
    let api_server = api::start_server(
        SocketAddr::from_str(&format!("0.0.0.0:{}", cfg.api_port))?,
        tls_config(&cfg.api_ca_crt, &cfg.api_crt, &cfg.api_key)?,
        api::State::new(engine, rpc_client),
        ct,
    );

    tt.spawn(async move {
        if let Err(e) = api_server.await {
            se.set_public_api_error(Some(e.to_string()));
        }
    });

    Ok(())
}
