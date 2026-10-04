use thiserror::Error;

#[derive(Debug, Error)]
pub enum MetricsError {
    #[error("a metrics recorder is already installed")]
    RecorderAlreadyInstalled,
}
