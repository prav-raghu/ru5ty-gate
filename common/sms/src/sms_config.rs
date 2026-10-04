use ru5ty_gate_config::EnvReader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmsConfig {
    pub enabled: bool,
    pub client_id: Option<String>,
    pub api_secret: Option<String>,
    pub sender_id: Option<String>,
}

impl SmsConfig {
    pub fn from_env(env: &EnvReader) -> Self {
        Self {
            enabled: env
                .optional("SMSPORTAL_ENABLED")
                .is_none_or(|value| value != "false"),
            client_id: env.optional("SMSPORTAL_CLIENT_ID"),
            api_secret: env.optional("SMSPORTAL_API_SECRET"),
            sender_id: env.optional("SMSPORTAL_SENDER_ID"),
        }
    }
}
