use crate::entrypoint;
use crate::picodata::rpc;
use picoplugin::interplay::channel::oneshot;
use picoplugin::plugin::interface::Service as PicoService;
use picoplugin::plugin::prelude::service_registrar;
use picoplugin::plugin::prelude::CallbackResult;
use picoplugin::plugin::prelude::PicoContext;
use picoplugin::plugin::prelude::ServiceRegistry;
use serde::Deserialize;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct Service {
    sw: ServiceWarnings,
    ct: CancellationToken,
    done_rx: Option<oneshot::EndpointReceiver<()>>,
}

#[derive(Debug, Deserialize)]
pub struct ServiceConfig {
    pub api_port: NonZeroUsize,
    pub api_ca_crt: PathBuf,
    pub api_crt: PathBuf,
    pub api_key: PathBuf,
    pub data_dir: PathBuf,
}

#[derive(Clone, Default)]
pub struct ServiceWarnings(Arc<Mutex<ServiceWarningsInner>>);

#[derive(Default)]
struct ServiceWarningsInner {
    private_api_server: Option<String>,
    public_api_server: Option<String>,
}

#[derive(Debug, Error)]
enum Error {
    #[error("rpc: {0}")]
    Rpc(#[from] rpc::Error),
    #[error("entrypoint: {0:?}")]
    Entrypoint(#[from] anyhow::Error),
}

impl PicoService for Service {
    type Config = ServiceConfig;

    fn on_start(&mut self, ctx: &PicoContext, cfg: Self::Config) -> CallbackResult<()> {
        let (done_tx, done_rx) = oneshot::channel::<()>();
        let rpc_client = rpc::spawn_proxy_server(ctx).map_err(|e| Error::Rpc(e))?;

        entrypoint(cfg, rpc_client, done_tx, self.ct.clone(), self.sw.clone())
            .map_err(|e| Error::Entrypoint(e))?;

        self.done_rx = Some(done_rx);
        Ok(())
    }

    fn on_stop(&mut self, _: &PicoContext) -> CallbackResult<()> {
        self.ct.cancel();

        if let Some(done_rx) = self.done_rx.take() {
            done_rx.receive().ok();
        }

        Ok(())
    }

    fn on_health_check(&self, _: &PicoContext) -> CallbackResult<()> {
        self.sw.check()
    }
}

impl ServiceWarnings {
    pub fn set_private_api_error(&self, e: Option<String>) {
        self.0.lock().unwrap().private_api_server = e;
    }

    pub fn set_public_api_error(&self, e: Option<String>) {
        self.0.lock().unwrap().public_api_server = e;
    }

    fn check(&self) -> CallbackResult<()> {
        let mut errors = Vec::new();
        let guard = self.0.lock().unwrap();

        if let Some(e) = &guard.private_api_server {
            errors.push(format!("private api server: {}", e));
        }

        if let Some(e) = &guard.public_api_server {
            errors.push(format!("public api server: {}", e));
        }

        if errors.is_empty() {
            return Ok(());
        }

        Err(errors.join(" && ").into())
    }
}

#[service_registrar]
fn register_service(registry: &mut ServiceRegistry) {
    registry.add(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        Service::default,
    )
}
