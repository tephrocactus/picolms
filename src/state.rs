use crate::engine::Engine;
use crate::service::SharedServiceErrors;
use std::sync::Arc;
use tokio_util::task::TaskTracker;

#[derive(Clone)]
pub struct SharedState(Arc<State>);

struct State {
    engine: Engine,
    se: SharedServiceErrors,
}

impl SharedState {
    pub fn new(engine: Engine, se: SharedServiceErrors) -> Self {
        Self(Arc::new(State { engine, se }))
    }

    pub fn engine(&self) -> &Engine {
        &self.0.engine
    }
}
