use ru5ty_gate_types::{Permission, TokenScope};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Access,
    Refresh,
    MfaChallenge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPayload {
    pub id: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<Permission>,
    pub scope: TokenScope,
    pub jti: String,
    #[serde(rename = "type")]
    pub token_type: TokenType,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaChallengePayload {
    pub id: String,
    #[serde(rename = "type")]
    pub token_type: TokenType,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_token_id: String,
}
