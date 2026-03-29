use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
pub struct Config {
    pub log_dir: String,
    pub database_url: String,
    pub registry_url: String,
    pub rate_limit_global_rpm: u64,
    pub rate_limit_per_key_rpm: u64,
    pub docs_dir: String,
    pub local_db_path: String,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, String> {
        let contents =
            std::fs::read_to_string(path).map_err(|e| format!("failed to read config: {e}"))?;
        toml::from_str(&contents).map_err(|e| format!("failed to parse config: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_config(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().expect("create temp file");
        f.write_all(content.as_bytes()).expect("write temp config");
        f.flush().expect("flush temp config");
        f
    }

    #[test]
    fn test_load_valid_config() {
        let toml = r#"
log_dir = "/tmp/logs"
database_url = "sqlite::memory:"
registry_url = "http://example.com/registry.txt"
rate_limit_global_rpm = 100
rate_limit_per_key_rpm = 10
docs_dir = "/tmp/docs"
local_db_path = "/tmp/local.db"
"#;
        let f = write_temp_config(toml);
        let cfg = Config::load(f.path()).expect("valid config should load");
        assert_eq!(cfg.log_dir, "/tmp/logs");
        assert_eq!(cfg.database_url, "sqlite::memory:");
        assert_eq!(cfg.registry_url, "http://example.com/registry.txt");
        assert_eq!(cfg.rate_limit_global_rpm, 100);
        assert_eq!(cfg.rate_limit_per_key_rpm, 10);
        assert_eq!(cfg.docs_dir, "/tmp/docs");
        assert_eq!(cfg.local_db_path, "/tmp/local.db");
    }

    #[test]
    fn test_load_missing_field_returns_error() {
        let toml = r#"
log_dir = "/tmp/logs"
"#;
        let f = write_temp_config(toml);
        let result = Config::load(f.path());
        assert!(result.is_err(), "missing required fields should fail");
        let err = result.unwrap_err();
        assert!(
            err.contains("failed to parse config"),
            "error should mention parsing: {err}"
        );
    }

    #[test]
    fn test_load_nonexistent_file_returns_error() {
        let result = Config::load(Path::new("/tmp/does-not-exist-12345.toml"));
        assert!(result.is_err(), "missing file should fail");
        let err = result.unwrap_err();
        assert!(
            err.contains("failed to read config"),
            "error should mention reading: {err}"
        );
    }

    #[test]
    fn test_load_invalid_toml_returns_error() {
        let f = write_temp_config("this is not valid toml {{{{");
        let result = Config::load(f.path());
        assert!(result.is_err(), "invalid TOML should fail");
    }

    #[test]
    fn test_load_wrong_types_returns_error() {
        let toml = r#"
log_dir = "/tmp/logs"
database_url = "sqlite::memory:"
registry_url = "http://example.com/registry.txt"
rate_limit_global_rpm = "not_a_number"
rate_limit_per_key_rpm = 10
docs_dir = "/tmp/docs"
local_db_path = "/tmp/local.db"
"#;
        let f = write_temp_config(toml);
        let result = Config::load(f.path());
        assert!(
            result.is_err(),
            "wrong type for rate_limit_global_rpm should fail"
        );
    }
}
