use ru5ty_gate_agent_config::ConfigError;
use ru5ty_gate_central_client::CentralError;
use ru5ty_gate_session_store::StoreError;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("failed to open session store at {path}: {source}")]
    Store {
        path: String,
        #[source]
        source: StoreError,
    },
    #[error("failed to build central platform client: {0}")]
    Central(#[from] CentralError),
    #[error("failed to bind {addr}: {source}")]
    Bind {
        addr: String,
        #[source]
        source: std::io::Error,
    },
    #[error("FAS server error: {0}")]
    Serve(#[source] std::io::Error),
    #[error("background task {task} stopped unexpectedly")]
    TaskStopped { task: &'static str },
}
