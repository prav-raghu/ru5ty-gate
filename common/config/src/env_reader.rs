use std::collections::HashMap;
use std::fmt::Display;
use std::str::FromStr;

use crate::config_error::ConfigError;

#[derive(Debug, Clone, Default)]
pub struct EnvReader {
    vars: HashMap<String, String>,
}

impl EnvReader {
    pub fn from_process() -> Self {
        Self {
            vars: std::env::vars().collect(),
        }
    }

    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self {
            vars: pairs
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }

    pub fn optional(&self, key: &str) -> Option<String> {
        self.vars
            .get(key)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    }

    pub fn required(&self, key: &str) -> Result<String, ConfigError> {
        self.optional(key).ok_or_else(|| ConfigError::Missing {
            key: key.to_owned(),
        })
    }

    pub fn parse_required<T>(&self, key: &str) -> Result<T, ConfigError>
    where
        T: FromStr,
        T::Err: Display,
    {
        let raw = self.required(key)?;
        parse_value(key, &raw)
    }

    pub fn parse_or<T>(&self, key: &str, default: T) -> Result<T, ConfigError>
    where
        T: FromStr,
        T::Err: Display,
    {
        match self.optional(key) {
            Some(raw) => parse_value(key, &raw),
            None => Ok(default),
        }
    }
}

fn parse_value<T>(key: &str, raw: &str) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: Display,
{
    raw.parse::<T>().map_err(|error| ConfigError::Invalid {
        key: key.to_owned(),
        reason: error.to_string(),
    })
}
