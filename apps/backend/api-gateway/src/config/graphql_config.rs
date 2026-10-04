use ru5ty_gate_config::EnvReader;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphqlConfig {
    pub enabled: bool,
    pub path: String,
    pub playground: bool,
    pub introspection: bool,
}

impl GraphqlConfig {
    pub fn from_env(env: &EnvReader, production: bool) -> Self {
        let flag = |key: &str| env.optional(key).is_some_and(|value| value == "true");
        Self {
            enabled: flag("GRAPHQL_ENABLED"),
            path: env
                .optional("GRAPHQL_PATH")
                .unwrap_or_else(|| "/graphql".to_owned()),
            playground: !production && flag("GRAPHQL_PLAYGROUND"),
            introspection: !production && flag("GRAPHQL_INTROSPECTION"),
        }
    }
}
