use rain_orderbook_common::raindex_client::local_db::pipeline::runner::scheduler::{
    start, NativeSyncHandle,
};
use std::path::PathBuf;

pub fn start_sync(settings_yaml: String, db_path: PathBuf) -> Result<NativeSyncHandle, String> {
    start(settings_yaml, db_path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_yaml_returns_error() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = start_sync("not valid yaml".into(), tmp.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_settings_returns_handle() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let yaml = rain_orderbook_app_settings::spec_version::CURRENT_SPEC_VERSION;
        let settings = format!(
            r#"
version: {yaml}
networks:
  testnet:
    rpcs:
      - https://rpc.example.com
    chain-id: 1337
subgraphs:
  testnet: https://subgraph.example.com/testnet
local-db-remotes:
  remote: https://remotes.example.com/manifest.yaml
local-db-sync:
  testnet:
    batch-size: 10
    max-concurrent-batches: 2
    retry-attempts: 3
    retry-delay-ms: 100
    rate-limit-delay-ms: 50
    finality-depth: 12
    bootstrap-block-threshold: 10000
    sync-interval-ms: 5000
orderbooks:
  book-a:
    address: 0x0000000000000000000000000000000000000001
    network: testnet
    subgraph: testnet
    local-db-remote: remote
    deployment-block: 0
"#
        );
        let result = start_sync(settings, tmp.path().to_path_buf());
        assert!(result.is_ok());
        result.unwrap().stop_and_join();
    }
}
