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
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[derive(Default)]
struct Service {
    rt: TokioExecutor<CBusTransport<'static>>,
    tt: TaskTracker,
    ct: CancellationToken,
}

#[derive(Debug, Deserialize)]
pub struct ServiceConfig {
    pub data_dir: PathBuf,
    pub private_api_port: NonZeroU16,
    pub public_api_port: NonZeroU16,
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
            ))
            .context("exec")?
            .context("entrypoint")?)
    }

    fn on_stop(&mut self, _: &PicoContext) -> CallbackResult<()> {
        self.ct.cancel();
        self.tt.close();
        Ok(self.rt.exec(self.tt.wait()).context("exec")?)
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
