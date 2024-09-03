use crate::engine::Engine;
use crate::picodata::rpc;
use std::sync::Arc;

#[derive(Clone)]
pub struct State(Arc<StateInner>);

struct StateInner {
    engine: Engine,
    rpc: rpc::ProxyClient,
}

impl State {
    pub fn new(engine: Engine, rpc: rpc::ProxyClient) -> Self {
        Self(Arc::new(StateInner { engine, rpc }))
    }

    pub fn engine(&self) -> &Engine {
        &self.0.engine
    }

    pub fn rpc(&self) -> &rpc::ProxyClient {
        &self.0.rpc
    }
}
