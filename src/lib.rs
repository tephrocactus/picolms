// mod accumulator;
mod api;
mod engine;
mod schema;
mod service;
mod state;
mod value;

use crate::engine::Engine;
use crate::service::ServiceConfig;
use crate::service::SharedServiceErrors;
use crate::state::SharedState;
use anyhow::Result;
use picoplugin::internal::types::InstanceInfo;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

async fn entrypoint(
    cfg: ServiceConfig,
    instance: InstanceInfo,
    tt: TaskTracker,
    ct: CancellationToken,
    se: SharedServiceErrors,
) -> Result<()> {
    let engine = Engine::new();
    let state = SharedState::new(engine, se.clone());

    let private_api_server = api::start_server(
        api::Config {
            addr: SocketAddr::from_str(&format!("0.0.0.0:{}", cfg.private_api_port))?,
            ca: cfg.private_api_ca,
            crt: cfg.private_api_crt,
            key: cfg.private_api_key,
        },
        api::private::router(state.clone()),
        ct.clone(),
    );

    let public_api_server = api::start_server(
        api::Config {
            addr: SocketAddr::from_str(&format!("0.0.0.0:{}", cfg.public_api_port))?,
            ca: cfg.public_api_ca,
            crt: cfg.public_api_crt,
            key: cfg.public_api_key,
        },
        api::public::router(state),
        ct,
    );

    tt.spawn(async move {
        select! {
            e = private_api_server => if let Err(e) = e {
                se.set_private_api_error(Some(e.to_string()));
            },

            e = public_api_server => if let Err(e) = e {
                se.set_public_api_error(Some(e.to_string()));
            }
        }
    });

    Ok(())
}
