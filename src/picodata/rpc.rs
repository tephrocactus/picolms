use picoplugin::interplay::channel::oneshot;
use picoplugin::interplay::channel::sync::std as channel;
use picoplugin::system::tarantool::cbus::RecvError;
use picoplugin::system::tarantool::error::Error as TarantoolError;
use picoplugin::system::tarantool::fiber;
use picoplugin::transport::rpc;
use std::time::Duration;
use thiserror::Error;
use tokio::task::spawn_blocking;
use tokio::task::JoinError;

const PROXY_CHANNEL_CAPACITY: usize = 100;

#[derive(Debug, Error)]
pub enum Error {
    #[error("spawn proxy: {0}")]
    ProxySpawn(#[from] TarantoolError),
    #[error("send proxy request: {0}")]
    ProxySend(String, ProxyRequest),
    #[error("receive proxy response: {0}")]
    ProxyReceive(#[from] RecvError),
    #[error("rpc request: {0}")]
    RpcRequest(String),
    #[error("tokio task join: {0}")]
    TokioTaskJoin(#[from] JoinError),
}

#[derive(Clone)]
pub struct ProxyClient(channel::Sender<ProxyMessage>);

#[derive(Debug)]
pub struct ProxyRequest {
    pub target: rpc::RequestTarget<'static>,
    pub path: String,
    pub data: rpc::Request<'static>,
    pub timeout: Duration,
}

struct ProxyMessage {
    request: ProxyRequest,
    response_tx: oneshot::Sender<Result<rpc::Response, Error>>,
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

pub fn spawn_proxy() -> Result<ProxyClient, Error> {
    let (tx, rx) = channel::channel(PROXY_CHANNEL_CAPACITY.try_into().unwrap());

    fiber::Builder::new()
        .func(move || proxy(rx))
        .start_non_joinable()?;

    Ok(ProxyClient(tx))
}

fn proxy(rx: channel::EndpointReceiver<ProxyMessage>) {
    while let Ok(msg) = rx.receive() {
        match rpc::RequestBuilder::new(msg.request.target)
            .path(&msg.request.path)
            .input(msg.request.data)
            .timeout(msg.request.timeout)
            .send()
        {
            Ok(response) => msg.response_tx.send(Ok(response)),
            Err(e) => msg.response_tx.send(Err(Error::RpcRequest(e.to_string()))),
        }
    }
}
