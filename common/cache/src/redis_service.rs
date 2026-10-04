use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::cache_error::CacheError;
use crate::redis_url::resolve_redis_url;

#[derive(Clone, Default)]
pub struct RedisService {
    manager: Option<ConnectionManager>,
}

impl RedisService {
    pub fn disabled() -> Self {
        Self { manager: None }
    }

    pub async fn connect(url: &str, reject_unauthorized: bool) -> Self {
        if url.starts_with("rediss://") && !reject_unauthorized {
            tracing::warn!(
                "Redis TLS certificate verification is disabled; set REDIS_TLS_REJECT_UNAUTHORIZED=true outside trusted networks"
            );
        }
        let resolved = resolve_redis_url(url, reject_unauthorized);
        let manager = match redis::Client::open(resolved) {
            Ok(client) => match ConnectionManager::new(client).await {
                Ok(manager) => {
                    tracing::info!("Redis connected successfully");
                    Some(manager)
                }
                Err(error) => {
                    tracing::warn!(%error, "Redis unavailable - running without cache");
                    None
                }
            },
            Err(error) => {
                tracing::warn!(%error, "Redis initialisation failed - running without cache");
                None
            }
        };
        Self { manager }
    }

    pub fn is_available(&self) -> bool {
        self.manager.is_some()
    }

    fn connection(&self) -> Result<ConnectionManager, CacheError> {
        self.manager.clone().ok_or(CacheError::Unavailable)
    }

    pub async fn ping(&self) -> Result<(), CacheError> {
        let mut connection = self.connection()?;
        let _: String = redis::cmd("PING").query_async(&mut connection).await?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        Ok(self.connection()?.get(key).await?)
    }

    pub async fn set_ex(&self, key: &str, ttl_seconds: u64, value: &str) -> Result<(), CacheError> {
        Ok(self.connection()?.set_ex(key, value, ttl_seconds).await?)
    }

    pub async fn del(&self, key: &str) -> Result<(), CacheError> {
        let _: i64 = self.connection()?.del(key).await?;
        Ok(())
    }

    pub async fn del_many(&self, keys: &[String]) -> Result<(), CacheError> {
        if keys.is_empty() {
            return Ok(());
        }
        let _: i64 = self.connection()?.del(keys).await?;
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        Ok(self.connection()?.exists(key).await?)
    }

    pub async fn incr(&self, key: &str) -> Result<i64, CacheError> {
        Ok(self.connection()?.incr(key, 1).await?)
    }

    pub async fn expire(&self, key: &str, ttl_seconds: i64) -> Result<(), CacheError> {
        let _: bool = self.connection()?.expire(key, ttl_seconds).await?;
        Ok(())
    }

    pub async fn keys_matching(&self, pattern: &str) -> Result<Vec<String>, CacheError> {
        let mut connection = self.connection()?;
        let mut iterator: redis::AsyncIter<String> = connection.scan_match(pattern).await?;
        let mut keys = Vec::new();
        while let Some(key) = iterator.next_item().await {
            keys.push(key?);
        }
        Ok(keys)
    }

    pub async fn get_json<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError> {
        match self.get(key).await? {
            Some(raw) => Ok(Some(serde_json::from_str(&raw)?)),
            None => Ok(None),
        }
    }

    pub async fn set_json<T: Serialize + Sync>(
        &self,
        key: &str,
        ttl_seconds: u64,
        value: &T,
    ) -> Result<(), CacheError> {
        self.set_ex(key, ttl_seconds, &serde_json::to_string(value)?)
            .await
    }
}
