use std::time::Duration;

use tokio::process::Command;

use crate::NdsctlError;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Default)]
pub struct Ndsctl {
    program: Option<String>,
}

fn is_valid_mac(mac: &str) -> bool {
    let octets: Vec<&str> = mac.split(':').collect();
    octets.len() == 6
        && octets
            .iter()
            .all(|octet| octet.len() == 2 && octet.chars().all(|c| c.is_ascii_hexdigit()))
}

impl Ndsctl {
    pub fn disabled() -> Self {
        Self { program: None }
    }

    pub fn with_program(program: impl Into<String>) -> Self {
        Self {
            program: Some(program.into()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.program.is_some()
    }

    pub async fn deauth(&self, mac: &str) -> Result<(), NdsctlError> {
        let Some(program) = &self.program else {
            return Ok(());
        };
        if !is_valid_mac(mac) {
            return Err(NdsctlError::InvalidMac);
        }
        let output = tokio::time::timeout(
            COMMAND_TIMEOUT,
            Command::new(program)
                .arg("deauth")
                .arg(mac)
                .kill_on_drop(true)
                .output(),
        )
        .await
        .map_err(|_| NdsctlError::TimedOut)?
        .map_err(NdsctlError::Spawn)?;
        if output.status.success() {
            tracing::debug!(mac, "ndsctl deauth succeeded");
            return Ok(());
        }
        Err(NdsctlError::Failed {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}
