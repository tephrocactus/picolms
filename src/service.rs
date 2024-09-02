use crate::entrypoint;
use anyhow::Context;
use picoplugin::internal::instance_info;
use picoplugin::interplay::tros::transport::cbus::CBusTransport;
use picoplugin::interplay::tros::TokioExecutor;
use picoplugin::plugin::interface::Service as PicoService;
use picoplugin::plugin::prelude::service_registrar;
use picoplugin::plugin::prelude::CallbackResult;
use picoplugin::plugin::prelude::PicoContext;
use picoplugin::plugin::prelude::ServiceRegistry;
use serde::Deserialize;
use std::num::NonZeroU16;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[derive(Default)]
struct Service {
    rt: TokioExecutor<CBusTransport<'static>>,
    tt: TaskTracker,
    ct: CancellationToken,
    se: SharedServiceErrors,
}

#[derive(Debug, Deserialize)]
pub struct ServiceConfig {
    pub data_dir: PathBuf,
    pub private_api_port: NonZeroU16,
    pub private_api_ca: PathBuf,
    pub private_api_crt: PathBuf,
    pub private_api_key: PathBuf,
    pub public_api_port: NonZeroU16,
    pub public_api_ca: PathBuf,
    pub public_api_crt: PathBuf,
    pub public_api_key: PathBuf,
}

#[derive(Clone, Default)]
pub struct SharedServiceErrors(Arc<Mutex<ServiceErrors>>);

#[derive(Default)]
struct ServiceErrors {
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
                instance_info().unwrap(),
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

impl SharedServiceErrors {
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
