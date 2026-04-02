use super::DbPool;

pub(crate) async fn get_setting(pool: &DbPool, key: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
}

pub(crate) async fn set_setting(pool: &DbPool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> DbPool {
        let id = uuid::Uuid::new_v4();
        crate::db::init(&format!("sqlite:file:{id}?mode=memory&cache=shared"))
            .await
            .expect("database init")
    }

    #[tokio::test]
    async fn test_get_setting_returns_none_for_missing_key() {
        let pool = test_pool().await;
        let result = get_setting(&pool, "nonexistent").await.expect("query");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_set_and_get_setting_round_trip() {
        let pool = test_pool().await;
        set_setting(&pool, "my_key", "my_value")
            .await
            .expect("set");
        let result = get_setting(&pool, "my_key").await.expect("get");
        assert_eq!(result, Some("my_value".to_string()));
    }

    #[tokio::test]
    async fn test_set_setting_upsert_overwrites_existing() {
        let pool = test_pool().await;
        set_setting(&pool, "k", "v1").await.expect("set first");
        set_setting(&pool, "k", "v2").await.expect("set second");
        let result = get_setting(&pool, "k").await.expect("get");
        assert_eq!(result, Some("v2".to_string()));
    }

    #[tokio::test]
    async fn test_different_keys_are_independent() {
        let pool = test_pool().await;
        set_setting(&pool, "a", "1").await.expect("set a");
        set_setting(&pool, "b", "2").await.expect("set b");

        assert_eq!(
            get_setting(&pool, "a").await.expect("get a"),
            Some("1".to_string())
        );
        assert_eq!(
            get_setting(&pool, "b").await.expect("get b"),
            Some("2".to_string())
        );
    }

    #[tokio::test]
    async fn test_set_setting_empty_value_stores_correctly() {
        let pool = test_pool().await;
        set_setting(&pool, "empty", "").await.expect("set empty");
        let result = get_setting(&pool, "empty").await.expect("get");
        assert_eq!(result, Some("".to_string()));
    }
}
