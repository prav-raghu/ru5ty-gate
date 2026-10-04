use ru5ty_gate_config::{ConfigError, EnvReader};

const DEFAULT_FROM_NAME: &str = "Ru5ty Gate";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmailConfig {
    pub api_key: Option<String>,
    pub from_email: Option<String>,
    pub from_name: String,
    pub test_inbox_id: Option<u64>,
}

impl EmailConfig {
    pub fn from_env(env: &EnvReader) -> Result<Self, ConfigError> {
        let test_inbox_id = match env.optional("MAILTRAP_TEST_INBOX_ID") {
            Some(raw) => Some(
                env.parse_required::<u64>("MAILTRAP_TEST_INBOX_ID")
                    .map_err(|_| ConfigError::Invalid {
                        key: "MAILTRAP_TEST_INBOX_ID".to_owned(),
                        reason: format!("expected a number, received {raw}"),
                    })?,
            ),
            None => None,
        };
        Ok(Self {
            api_key: env.optional("MAILTRAP_API_KEY"),
            from_email: env.optional("MAILTRAP_FROM"),
            from_name: env
                .optional("MAILTRAP_FROM_NAME")
                .unwrap_or_else(|| DEFAULT_FROM_NAME.to_owned()),
            test_inbox_id,
        })
    }

    pub fn send_url(&self) -> String {
        match self.test_inbox_id {
            Some(inbox) => format!("https://sandbox.api.mailtrap.io/api/send/{inbox}"),
            None => "https://send.api.mailtrap.io/api/send".to_owned(),
        }
    }
}
