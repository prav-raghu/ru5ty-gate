use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FasError {
    MissingPayload,
    MalformedPayload,
    InvalidField(&'static str),
    UnexpectedGateway,
    UntrustedPeer,
}

impl FasError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::UntrustedPeer => StatusCode::FORBIDDEN,
            _ => StatusCode::BAD_REQUEST,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            Self::MissingPayload => {
                "missing fas payload: set fas_secure_enabled to 1 in the openNDS config"
            }
            Self::MalformedPayload => "fas payload could not be decoded",
            Self::InvalidField(_) => "fas payload contains an invalid field",
            Self::UnexpectedGateway => "unrecognized gateway for this agent instance",
            Self::UntrustedPeer => "request does not come from the client it describes",
        }
    }
}

impl std::fmt::Display for FasError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField(field) => write!(formatter, "invalid field {field}"),
            other => formatter.write_str(other.message()),
        }
    }
}

impl std::error::Error for FasError {}

impl IntoResponse for FasError {
    fn into_response(self) -> Response {
        (self.status(), self.message()).into_response()
    }
}
