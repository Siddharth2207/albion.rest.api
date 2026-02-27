use rain_orderbook_app_settings::local_db_manifest::DB_SCHEMA_VERSION;
use rain_orderbook_common::local_db::executor::RusqliteExecutor;
use rain_orderbook_common::local_db::pipeline::runner::utils::parse_runner_settings;
use rain_orderbook_common::raindex_client::local_db::pipeline::bootstrap::ClientBootstrapAdapter;
use rain_orderbook_common::raindex_client::local_db::pipeline::runner::config::NetworkRunnerConfig;
use rain_orderbook_common::raindex_client::local_db::pipeline::runner::environment::default_environment;
use rain_orderbook_common::raindex_client::local_db::pipeline::runner::leadership::DefaultLeadership;
use rain_orderbook_common::raindex_client::local_db::pipeline::runner::ClientRunner;
use std::path::PathBuf;
use std::time::Duration;

use rain_orderbook_common::local_db::pipeline::adapters::bootstrap::BootstrapPipeline;

const SYNC_INTERVAL: Duration = Duration::from_secs(5);

pub fn start_sync(settings_yaml: String, db_path: PathBuf) {
    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!(error = %e, "failed to build sync runtime");
                return;
            }
        };

        runtime.block_on(async move {
            run_sync(settings_yaml, db_path).await;
        });
    });
}

async fn run_sync(settings_yaml: String, db_path: PathBuf) {
    let executor = RusqliteExecutor::new(&db_path);

    let bootstrap = ClientBootstrapAdapter::new();
    if let Err(e) = bootstrap
        .runner_run(&executor, Some(DB_SCHEMA_VERSION))
        .await
    {
        tracing::error!(error = %e, "failed to bootstrap local db schema");
        return;
    }
    tracing::info!("local db schema bootstrapped");

    let settings = match parse_runner_settings(&settings_yaml) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "failed to parse runner settings");
            return;
        }
    };

    let mut network_keys: Vec<String> = Vec::new();
    for ob in settings.orderbooks.values() {
        if !network_keys.contains(&ob.network.key) {
            network_keys.push(ob.network.key.clone());
        }
    }
    network_keys.sort();

    if network_keys.is_empty() {
        tracing::warn!("no networks found in settings, sync will not run");
        return;
    }

    tracing::info!(networks = ?network_keys, "starting local db sync");

    let local_set = tokio::task::LocalSet::new();
    local_set
        .run_until(async {
            for network_key in &network_keys {
                let config =
                    match NetworkRunnerConfig::from_global_settings(&settings, network_key) {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::error!(network = %network_key, error = %e, "failed to build network config");
                            continue;
                        }
                    };

                let leadership = DefaultLeadership::new();
                let environment = default_environment();

                let mut runner = match ClientRunner::from_config(config, environment, leadership) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!(network = %network_key, error = %e, "failed to create runner");
                        continue;
                    }
                };

                let db_path = db_path.clone();
                tokio::task::spawn_local(async move {
                    loop {
                        let executor = RusqliteExecutor::new(&db_path);
                        match runner.run(&executor).await {
                            Ok(outcome) => {
                                tracing::debug!(
                                    chain_id = runner.chain_id(),
                                    network = runner.network_key(),
                                    outcome = ?outcome,
                                    "sync cycle complete"
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    chain_id = runner.chain_id(),
                                    network = runner.network_key(),
                                    error = %e,
                                    "sync cycle failed"
                                );
                            }
                        }
                        tokio::time::sleep(SYNC_INTERVAL).await;
                    }
                });
            }

            std::future::pending::<()>().await;
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rain_orderbook_app_settings::spec_version::CURRENT_SPEC_VERSION;

    fn test_settings_yaml() -> String {
        format!(
            r#"
version: {version}
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
orderbooks:
  book-a:
    address: 0x0000000000000000000000000000000000000001
    network: testnet
    subgraph: testnet
    local-db-remote: remote
    deployment-block: 0
"#,
            version = CURRENT_SPEC_VERSION
        )
    }

    fn multi_network_yaml() -> String {
        format!(
            r#"
version: {version}
networks:
  alpha:
    rpcs:
      - https://rpc-alpha.example.com
    chain-id: 1
  beta:
    rpcs:
      - https://rpc-beta.example.com
    chain-id: 2
subgraphs:
  alpha: https://subgraph.example.com/alpha
  beta: https://subgraph.example.com/beta
local-db-remotes:
  remote: https://remotes.example.com/manifest.yaml
local-db-sync:
  alpha:
    batch-size: 10
    max-concurrent-batches: 2
    retry-attempts: 3
    retry-delay-ms: 100
    rate-limit-delay-ms: 50
    finality-depth: 12
    bootstrap-block-threshold: 10000
  beta:
    batch-size: 10
    max-concurrent-batches: 2
    retry-attempts: 3
    retry-delay-ms: 100
    rate-limit-delay-ms: 50
    finality-depth: 12
    bootstrap-block-threshold: 10000
orderbooks:
  book-a:
    address: 0x0000000000000000000000000000000000000001
    network: alpha
    subgraph: alpha
    local-db-remote: remote
    deployment-block: 0
  book-b:
    address: 0x0000000000000000000000000000000000000002
    network: beta
    subgraph: beta
    local-db-remote: remote
    deployment-block: 0
"#,
            version = CURRENT_SPEC_VERSION
        )
    }

    fn empty_orderbooks_yaml() -> String {
        format!(
            r#"
version: {version}
networks:
  testnet:
    rpcs:
      - https://rpc.example.com
    chain-id: 1337
subgraphs:
  testnet: https://subgraph.example.com/testnet
local-db-sync:
  testnet:
    batch-size: 10
    max-concurrent-batches: 2
    retry-attempts: 3
    retry-delay-ms: 100
    rate-limit-delay-ms: 50
    finality-depth: 12
    bootstrap-block-threshold: 10000
"#,
            version = CURRENT_SPEC_VERSION
        )
    }

    fn extract_network_keys(yaml: &str) -> Vec<String> {
        let settings = parse_runner_settings(yaml).expect("should parse");
        let mut keys: Vec<String> = Vec::new();
        for ob in settings.orderbooks.values() {
            if !keys.contains(&ob.network.key) {
                keys.push(ob.network.key.clone());
            }
        }
        keys.sort();
        keys
    }

    #[tokio::test]
    async fn test_bootstrap_creates_schema() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        let executor = RusqliteExecutor::new(&path);
        let bootstrap = ClientBootstrapAdapter::new();
        bootstrap
            .runner_run(&executor, Some(DB_SCHEMA_VERSION))
            .await
            .expect("bootstrap should succeed");

        assert!(path.exists());
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
    }

    #[test]
    fn test_parse_settings_extracts_networks() {
        let yaml = test_settings_yaml();
        let keys = extract_network_keys(&yaml);
        assert_eq!(keys, vec!["testnet"]);
    }

    #[test]
    fn test_parse_settings_multiple_networks() {
        let yaml = multi_network_yaml();
        let keys = extract_network_keys(&yaml);
        assert_eq!(keys, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_parse_settings_missing_orderbooks_returns_error() {
        let yaml = empty_orderbooks_yaml();
        let result = parse_runner_settings(&yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_settings_deduplicates_networks() {
        let yaml = format!(
            r#"
version: {version}
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
orderbooks:
  book-a:
    address: 0x0000000000000000000000000000000000000001
    network: testnet
    subgraph: testnet
    local-db-remote: remote
    deployment-block: 0
  book-b:
    address: 0x0000000000000000000000000000000000000002
    network: testnet
    subgraph: testnet
    local-db-remote: remote
    deployment-block: 0
"#,
            version = CURRENT_SPEC_VERSION
        );
        let keys = extract_network_keys(&yaml);
        assert_eq!(keys, vec!["testnet"]);
    }

    #[test]
    fn test_network_config_from_settings() {
        let yaml = test_settings_yaml();
        let settings = parse_runner_settings(&yaml).expect("should parse");
        NetworkRunnerConfig::from_global_settings(&settings, "testnet")
            .expect("should build network config");
    }

    #[test]
    fn test_runner_creation() {
        let yaml = test_settings_yaml();
        let settings = parse_runner_settings(&yaml).expect("should parse");
        let config = NetworkRunnerConfig::from_global_settings(&settings, "testnet")
            .expect("should build config");
        ClientRunner::from_config(config, default_environment(), DefaultLeadership::new())
            .expect("should create runner");
    }

    #[test]
    fn test_invalid_yaml_returns_error() {
        let result = parse_runner_settings("not valid settings yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_nonexistent_network_returns_error() {
        let yaml = test_settings_yaml();
        let settings = parse_runner_settings(&yaml).expect("should parse");
        let result = NetworkRunnerConfig::from_global_settings(&settings, "nonexistent");
        assert!(result.is_err());
    }
}
