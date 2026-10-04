use std::str::FromStr;

use crate::StoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncEventKind {
    SessionStart,
    SessionEnd,
}

impl SyncEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "session_start",
            Self::SessionEnd => "session_end",
        }
    }
}

impl FromStr for SyncEventKind {
    type Err = StoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "session_start" => Ok(Self::SessionStart),
            "session_end" => Ok(Self::SessionEnd),
            other => Err(StoreError::UnknownEventKind(other.to_owned())),
        }
    }
}
