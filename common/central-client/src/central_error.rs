#[derive(Debug, thiserror::Error)]
pub enum CentralError {
    #[error("failed to build http client: {0}")]
    ClientBuild(#[source] reqwest::Error),
    #[error("request to central API failed: {0}")]
    Request(#[source] reqwest::Error),
    #[error("central API returned {status}: {body}")]
    Api { status: u16, body: String },
}

pub type Result<T> = std::result::Result<T, CentralError>;
