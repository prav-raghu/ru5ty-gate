use std::sync::LazyLock;

const BCRYPT_COST: u32 = 10;

static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| bcrypt::hash("timing-equaliser", BCRYPT_COST).unwrap_or_default());

pub struct PasswordUtil;

impl PasswordUtil {
    pub async fn hash(password: &str) -> Option<String> {
        Self::hash_with_cost(password, BCRYPT_COST).await
    }

    pub async fn hash_with_cost(password: &str, cost: u32) -> Option<String> {
        let password = password.to_owned();
        tokio::task::spawn_blocking(move || bcrypt::hash(password, cost).ok())
            .await
            .ok()
            .flatten()
    }

    pub async fn verify(password: &str, hash: &str) -> bool {
        let password = password.to_owned();
        let hash = hash.to_owned();
        tokio::task::spawn_blocking(move || bcrypt::verify(password, &hash).unwrap_or(false))
            .await
            .unwrap_or(false)
    }

    pub async fn verify_or_dummy(password: &str, hash: Option<&str>) -> bool {
        match hash {
            Some(hash) => Self::verify(password, hash).await,
            None => {
                let _ = Self::verify(password, &DUMMY_HASH).await;
                false
            }
        }
    }
}
