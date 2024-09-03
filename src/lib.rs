mod api;
mod engine;
pub(crate) mod picodata;

use crate::api::tls_config;
use crate::engine::Engine;
use crate::picodata::rpc::ProxyClient;
use crate::picodata::service::ServiceConfig;
use crate::picodata::service::ServiceWarnings;
use anyhow::Result;
use picoplugin::interplay::channel::oneshot;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio_util::sync::CancellationToken;

pub fn entrypoint(
    cfg: ServiceConfig,
    rpc_client: ProxyClient,
    done_tx: oneshot::Sender<()>,
    ct: CancellationToken,
    sw: ServiceWarnings,
) -> Result<()> {
    let engine = Engine::new();
    let api_server = api::start_server(
        SocketAddr::from_str(&format!("0.0.0.0:{}", cfg.api_port))?,
        tls_config(&cfg.api_ca_crt, &cfg.api_crt, &cfg.api_key)?,
        api::State::new(engine, rpc_client),
        ct,
    );

    std::thread::spawn(move || {
        if let Err(e) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(api_server)
        {
            sw.set_public_api_error(Some(e.to_string()));
        }

        done_tx.send(());
    });

    Ok(())
}
