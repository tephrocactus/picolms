use picoplugin::internal::instance_info;
use picoplugin::internal::types::InstanceInfo;
use picoplugin::interplay::channel::oneshot;
use picoplugin::interplay::channel::sync::std as channel;
use picoplugin::plugin::prelude::PicoContext;
use picoplugin::system::tarantool::cbus::RecvError;
use picoplugin::system::tarantool::error::BoxError;
use picoplugin::system::tarantool::error::Error as TarantoolError;
use picoplugin::system::tarantool::fiber;
use picoplugin::transport::context::Context;
use picoplugin::transport::rpc;
use std::time::Duration;
use thiserror::Error;
use tokio::task::spawn_blocking;
use tokio::task::JoinError;

const PROXY_CHANNEL_CAPACITY: usize = 100;

#[derive(Clone)]
pub struct ProxyClient(channel::Sender<ProxyMessage>);

#[derive(Debug, Clone)]
pub struct ProxyRequest {
    pub target: rpc::RequestTarget<'static>,
    pub path: Path,
    pub data: Vec<u8>,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Copy)]
pub enum Path {
    Insert,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("spawn proxy server: {0}")]
    ProxySpawn(#[from] TarantoolError),
    #[error("register server: {0}")]
    ServerRegister(String),
    #[error("get instance info: {0}")]
    InstanceInfo(String),
    #[error("send proxy request: {0}")]
    ProxySend(String, ProxyRequest),
    #[error("receive proxy response: {0}")]
    ProxyReceive(#[from] RecvError),
    #[error("request: {0}")]
    Request(String),
    #[error("tokio task join: {0}")]
    TokioTaskJoin(#[from] JoinError),
}

struct ProxyServer;

struct ProxyMessage {
    request: ProxyRequest,
    response_tx: oneshot::Sender<Result<rpc::Response, Error>>,
}

struct ServiceInfo {
    name: String,
    plugin_name: String,
    plugin_version: String,
}

impl ProxyClient {
    pub fn send_sync(&self, request: ProxyRequest) -> Result<rpc::Response, Error> {
        Self::send(&self.0, request)
    }

    pub async fn send_async(&self, request: ProxyRequest) -> Result<rpc::Response, Error> {
        let tx = self.0.clone();
        spawn_blocking(move || Self::send(&tx, request)).await?
    }

    fn send(
        tx: &channel::Sender<ProxyMessage>,
        request: ProxyRequest,
    ) -> Result<rpc::Response, Error> {
        let (response_tx, response_rx) = oneshot::channel();

        tx.send(ProxyMessage {
            request,
            response_tx,
        })
        .map_err(|e| Error::ProxySend(e.to_string(), e.0.request))?;

        response_rx.receive()?
    }
}

impl Path {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Insert => "/insert",
        }
    }
}

impl From<&PicoContext> for ServiceInfo {
    fn from(ctx: &PicoContext) -> Self {
        Self {
            plugin_name: ctx.plugin_name().to_string(),
            plugin_version: ctx.plugin_version().to_string(),
            name: ctx.service_name().to_string(),
        }
    }
}

impl ProxyServer {
    fn run(rx: channel::EndpointReceiver<ProxyMessage>, _: InstanceInfo, service: ServiceInfo) {
        while let Ok(msg) = rx.receive() {
            match rpc::RequestBuilder::new(msg.request.target)
                .plugin_service(&service.plugin_name, &service.name)
                .plugin_version(&service.plugin_version)
                .path(msg.request.path.as_str())
                .input(rpc::Request::from_bytes(&msg.request.data))
                .timeout(msg.request.timeout)
                .send()
            {
                Ok(response) => msg.response_tx.send(Ok(response)),
                Err(e) => msg.response_tx.send(Err(Error::Request(e.to_string()))),
            }
        }
    }
}

pub fn spawn_proxy_server(ctx: &PicoContext) -> Result<ProxyClient, Error> {
    let (tx, rx) = channel::channel(PROXY_CHANNEL_CAPACITY.try_into().unwrap());
    let instance = instance_info().map_err(|e| Error::InstanceInfo(e.to_string()))?;
    let service = ServiceInfo::from(ctx);

    fiber::Builder::new()
        .func(move || ProxyServer::run(rx, instance, service))
        .start_non_joinable()?;

    Ok(ProxyClient(tx))
}

fn register_server<H>(ctx: &PicoContext, path: Path, handler: H) -> Result<(), Error>
where
    H: FnMut(rpc::Request<'_>, &mut Context) -> Result<rpc::Response, BoxError> + 'static,
{
    rpc::RouteBuilder::from_pico_context(ctx)
        .path(path.as_str())
        .register(handler)
        .map_err(|e| Error::ServerRegister(e.to_string()))
}
