#[derive(Debug, thiserror::Error)]
pub enum NdsctlError {
    #[error("refusing to run ndsctl with an invalid mac address")]
    InvalidMac,
    #[error("failed to run ndsctl: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("ndsctl timed out")]
    TimedOut,
    #[error("ndsctl exited with {code:?}: {stderr}")]
    Failed { code: Option<i32>, stderr: String },
}
