use crate::auth::AuthKeyId;
use crate::db::DbPool;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Data, Request, Response};
use std::time::Instant;

struct UsageStart(Instant);

pub struct UsageLogger;

#[rocket::async_trait]
impl Fairing for UsageLogger {
    fn info(&self) -> Info {
        Info {
            name: "Usage Logger",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        req.local_cache(|| UsageStart(Instant::now()));
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let api_key_id = match req.local_cache(|| AuthKeyId(None)).0 {
            Some(id) => id,
            None => return,
        };

        let pool = match req.rocket().state::<DbPool>() {
            Some(p) => p.clone(),
            None => return,
        };

        let start = &req.local_cache(|| UsageStart(Instant::now())).0;
        let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
        let method = req.method().as_str().to_owned();
        let path = req.uri().path().to_string();
        let status_code = res.status().code as i32;

        tokio::spawn(async move {
            if let Err(e) = sqlx::query(
                "INSERT INTO usage_logs (api_key_id, method, path, status_code, latency_ms) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(api_key_id)
            .bind(&method)
            .bind(&path)
            .bind(status_code)
            .bind(latency_ms)
            .execute(&pool)
            .await
            {
                tracing::error!(error = %e, "failed to insert usage log");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::{basic_auth_header, client, seed_api_key, TestClientBuilder};
    use rocket::http::{Header, Status};

    /// Poll the usage_logs table until `expected` rows exist or timeout (2s).
    /// Replaces brittle `sleep(100ms)` with a retry loop that tolerates CI load.
    async fn await_usage_log_count(pool: &crate::db::DbPool, expected: i64) {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM usage_logs")
                .fetch_one(pool)
                .await
                .expect("query usage_logs count");
            if row.0 == expected {
                return;
            }
            if tokio::time::Instant::now() >= deadline {
                panic!(
                    "timed out waiting for usage_logs count to reach {expected}, got {}",
                    row.0
                );
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    #[rocket::async_test]
    async fn test_authenticated_request_creates_usage_log() {
        let client = client().await;
        let (key_id, secret) = seed_api_key(&client).await;
        let header = basic_auth_header(&key_id, &secret);

        client
            .get("/v1/tokens")
            .header(Header::new("Authorization", header))
            .dispatch()
            .await;

        let pool = client.rocket().state::<crate::db::DbPool>().expect("pool");
        await_usage_log_count(pool, 1).await;

        let log: (i64, String, String) =
            sqlx::query_as("SELECT api_key_id, method, path FROM usage_logs LIMIT 1")
                .fetch_one(pool)
                .await
                .expect("query");

        let api_key: (i64,) = sqlx::query_as("SELECT id FROM api_keys WHERE key_id = ?")
            .bind(&key_id)
            .fetch_one(pool)
            .await
            .expect("query");

        assert_eq!(log.0, api_key.0);
        assert_eq!(log.1, "GET");
        assert_eq!(log.2, "/v1/tokens");
    }

    #[rocket::async_test]
    async fn test_unauthenticated_request_creates_no_usage_log() {
        let client = client().await;

        let response = client.get("/health").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        // No async log task spawned for unauthenticated requests, but give a
        // brief window so any accidental spawn would be caught.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let pool = client.rocket().state::<crate::db::DbPool>().expect("pool");
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM usage_logs")
            .fetch_one(pool)
            .await
            .expect("query");
        assert_eq!(row.0, 0);
    }

    #[rocket::async_test]
    async fn test_failed_auth_creates_no_usage_log() {
        let client = client().await;
        let (key_id, _) = seed_api_key(&client).await;
        let header = basic_auth_header(&key_id, "wrong-secret");

        let response = client
            .get("/v1/tokens")
            .header(Header::new("Authorization", header))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized);

        // No async log task spawned for failed auth, brief window to catch bugs.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let pool = client.rocket().state::<crate::db::DbPool>().expect("pool");
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM usage_logs")
            .fetch_one(pool)
            .await
            .expect("query");
        assert_eq!(row.0, 0);
    }

    #[rocket::async_test]
    async fn test_rate_limited_authenticated_request_is_logged() {
        let rl = crate::fairings::RateLimiter::new(10000, 1);
        let client = TestClientBuilder::new().rate_limiter(rl).build().await;

        let (key_id, secret) = seed_api_key(&client).await;
        let header = basic_auth_header(&key_id, &secret);

        let first = client
            .get("/v1/tokens")
            .header(Header::new("Authorization", header.clone()))
            .dispatch()
            .await;
        assert_ne!(first.status(), Status::Unauthorized);
        assert_ne!(first.status(), Status::TooManyRequests);

        let second = client
            .get("/v1/tokens")
            .header(Header::new("Authorization", header))
            .dispatch()
            .await;
        assert_eq!(second.status(), Status::TooManyRequests);

        let pool = client.rocket().state::<crate::db::DbPool>().expect("pool");
        await_usage_log_count(pool, 2).await;

        let api_key: (i64,) = sqlx::query_as("SELECT id FROM api_keys WHERE key_id = ?")
            .bind(&key_id)
            .fetch_one(pool)
            .await
            .expect("query");

        let limited_rows: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM usage_logs WHERE api_key_id = ? AND status_code = 429",
        )
        .bind(api_key.0)
        .fetch_one(pool)
        .await
        .expect("query");
        assert_eq!(limited_rows.0, 1);
    }

    #[rocket::async_test]
    async fn test_global_rate_limited_unauthenticated_requests_create_no_usage_log() {
        let rl = crate::fairings::RateLimiter::new(1, 10000);
        let client = TestClientBuilder::new().rate_limiter(rl).build().await;

        let first = client.get("/v1/tokens").dispatch().await;
        assert_eq!(first.status(), Status::Unauthorized);

        let second = client.get("/v1/tokens").dispatch().await;
        assert_eq!(second.status(), Status::TooManyRequests);

        // No auth succeeded so no logs should exist; brief window to catch bugs.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let pool = client.rocket().state::<crate::db::DbPool>().expect("pool");
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM usage_logs")
            .fetch_one(pool)
            .await
            .expect("query");
        assert_eq!(row.0, 0);
    }
}
