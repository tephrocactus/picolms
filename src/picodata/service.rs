use crate::entrypoint;
use crate::picodata::rpc;
use anyhow::Context;
use picoplugin::interplay::tros::transport::cbus::CBusTransport;
use picoplugin::interplay::tros::TokioExecutor;
use picoplugin::plugin::interface::Service as PicoService;
use picoplugin::plugin::prelude::service_registrar;
use picoplugin::plugin::prelude::CallbackResult;
use picoplugin::plugin::prelude::PicoContext;
use picoplugin::plugin::prelude::ServiceRegistry;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[derive(Default)]
struct Service {
    rt: TokioExecutor<CBusTransport<'static>>,
    tt: TaskTracker,
    se: ServiceErrors,
    ct: CancellationToken,
}

#[derive(Debug, Deserialize)]
pub struct ServiceConfig {
    pub api_port: u16,
    pub api_ca_crt: PathBuf,
    pub api_crt: PathBuf,
    pub api_key: PathBuf,
    pub data_dir: PathBuf,
}

#[derive(Clone, Default)]
pub struct ServiceErrors(Arc<Mutex<ServiceErrorsInner>>);

#[derive(Default)]
struct ServiceErrorsInner {
    private_api_server: Option<String>,
    public_api_server: Option<String>,
}

impl PicoService for Service {
    type Config = ServiceConfig;

    fn on_start(&mut self, _: &PicoContext, config: Self::Config) -> CallbackResult<()> {
        Ok(self
            .rt
            .exec(entrypoint(
                config,
                rpc::spawn_proxy().context("spawn rpc proxy")?,
                self.tt.clone(),
                self.ct.clone(),
                self.se.clone(),
            ))
            .context("exec")?
            .context("entrypoint")?)
    }

    fn on_stop(&mut self, _: &PicoContext) -> CallbackResult<()> {
        self.ct.cancel();
        self.tt.close();
        Ok(self.rt.exec(self.tt.wait()).context("exec")?)
    }

    fn on_health_check(&self, _: &PicoContext) -> CallbackResult<()> {
        self.se.check()
    }
}

impl ServiceErrors {
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
