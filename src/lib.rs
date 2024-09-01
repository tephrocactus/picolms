mod accumulator;
mod block;
mod partition;
mod service;
mod table;

use crate::service::ServiceConfig;
use anyhow::Result;
use picoplugin::internal::types::InstanceInfo;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

async fn entrypoint(
    config: ServiceConfig,
    instance: InstanceInfo,
    tt: TaskTracker,
    ct: CancellationToken,
) -> Result<()> {
    todo!()
}
