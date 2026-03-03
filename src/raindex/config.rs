use crate::error::ApiError;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct RaindexProvider {
    registry_url: String,
}

impl RaindexProvider {
    pub(crate) async fn load(registry_url: &str) -> Result<Self, RaindexProviderError> {
        let url = registry_url.to_string();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| RaindexProviderError::RegistryLoad(e.to_string()))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| RaindexProviderError::RegistryLoad(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RaindexProviderError::RegistryLoad(format!(
                "registry returned {}",
                response.status()
            )));
        }

        Ok(Self { registry_url: url })
    }

    pub(crate) fn registry_url(&self) -> String {
        self.registry_url.clone()
    }

    pub(crate) async fn run_with_client<T, F, Fut>(&self, _f: F) -> Result<T, RaindexProviderError>
    where
        T: Send + 'static,
        F: FnOnce(()) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = T>,
    {
        Err(RaindexProviderError::ClientInit(
            "raindex client not available (stub)".into(),
        ))
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RaindexProviderError {
    #[error("failed to load registry: {0}")]
    RegistryLoad(String),
    #[error("failed to initialize registry runtime")]
    RegistryRuntimeInit(#[source] std::io::Error),
    #[error("failed to create raindex client: {0}")]
    ClientInit(String),
    #[error("failed to initialize client runtime")]
    ClientRuntimeInit(#[source] std::io::Error),
    #[error("worker thread panicked")]
    WorkerPanicked,
}

impl From<RaindexProviderError> for ApiError {
    fn from(e: RaindexProviderError) -> Self {
        tracing::error!(error = %e, "raindex client provider error");
        match e {
            RaindexProviderError::RegistryLoad(_)
            | RaindexProviderError::RegistryRuntimeInit(_) => {
                ApiError::Internal("registry configuration error".into())
            }
            RaindexProviderError::ClientInit(_) => {
                ApiError::Internal("failed to initialize orderbook client".into())
            }
            RaindexProviderError::ClientRuntimeInit(_) | RaindexProviderError::WorkerPanicked => {
                ApiError::Internal("failed to initialize client runtime".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rocket::async_test]
    async fn test_load_fails_with_unreachable_url() {
        let result = RaindexProvider::load("http://127.0.0.1:1/registry.txt").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RaindexProviderError::RegistryLoad(_)
        ));
    }

    #[rocket::async_test]
    async fn test_load_succeeds_with_200_response() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let body = "http://example.com/settings.yaml";
        let response = format!(
            "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let _ = tokio::io::AsyncReadExt::read(&mut socket, &mut buf).await;
            let _ = tokio::io::AsyncWriteExt::write_all(&mut socket, response.as_bytes()).await;
        });

        let result = RaindexProvider::load(&format!("http://{addr}/registry.txt")).await;
        assert!(result.is_ok());
        let provider = result.unwrap();
        assert_eq!(provider.registry_url(), format!("http://{addr}/registry.txt"));
    }

    #[rocket::async_test]
    async fn test_run_with_client_returns_error_in_stub() {
        let config = crate::test_helpers::mock_raindex_config().await;
        let result = config.run_with_client(|_| async { "ok" }).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RaindexProviderError::ClientInit(_)
        ));
    }

    #[test]
    fn test_error_maps_to_api_error() {
        let err = RaindexProviderError::RegistryLoad("test".into());
        let api_err: ApiError = err.into();
        assert!(
            matches!(api_err, ApiError::Internal(msg) if msg == "registry configuration error")
        );

        let err = RaindexProviderError::ClientInit("test".into());
        let api_err: ApiError = err.into();
        assert!(
            matches!(api_err, ApiError::Internal(msg) if msg == "failed to initialize orderbook client")
        );
    }
}
