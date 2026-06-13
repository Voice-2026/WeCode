use anyhow::{Context, bail};
use clap::Parser;
use serde::Deserialize;
use std::{
    env, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

const DEFAULT_CONFIG_PATH: &str = "config.toml";

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(long, env = "CODEX_SERVICE_CONFIG")]
    pub config: Option<PathBuf>,
    #[arg(long, env = "CODEX_SERVER_ADDR")]
    pub addr: Option<String>,
    #[arg(long = "db", env = "CODEX_SERVER_DB")]
    pub db_path: Option<PathBuf>,
    #[arg(long = "stats", env = "CODEX_STATS_ENABLED", num_args = 0..=1, default_missing_value = "true")]
    pub stats_enabled: Option<bool>,
    #[arg(long = "stats-path", env = "CODEX_STATS_PATH")]
    pub stats_path: Option<PathBuf>,
    #[arg(long = "stats-flush-interval", env = "CODEX_STATS_FLUSH_INTERVAL")]
    pub stats_flush_interval_secs: Option<u64>,
    #[arg(long = "pairing-ttl", env = "CODEX_PAIRING_TTL")]
    pub pairing_ttl_secs: Option<u64>,
    #[arg(long = "shutdown-timeout", env = "CODEX_SHUTDOWN_TIMEOUT")]
    pub shutdown_timeout_secs: Option<u64>,
    #[arg(long = "read-header-timeout", env = "CODEX_READ_HEADER_TIMEOUT")]
    pub read_header_timeout_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub addr: SocketAddr,
    pub db_path: PathBuf,
    pub stats_enabled: bool,
    pub stats_path: PathBuf,
    pub stats_flush_interval: Duration,
    pub pairing_ttl: Duration,
    pub shutdown_timeout: Duration,
    pub read_header_timeout: Duration,
    pub config_loaded_from: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    server: Option<FileServerConfig>,
    database: Option<FileDatabaseConfig>,
    stats: Option<FileStatsConfig>,
    pairing: Option<FilePairingConfig>,
    shutdown: Option<FileShutdownConfig>,
}

#[derive(Debug, Default, Deserialize)]
struct FileServerConfig {
    addr: Option<String>,
    read_header_timeout_seconds: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct FileDatabaseConfig {
    path: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct FileStatsConfig {
    enabled: Option<bool>,
    path: Option<PathBuf>,
    flush_interval_seconds: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct FilePairingConfig {
    ttl_seconds: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct FileShutdownConfig {
    timeout_seconds: Option<u64>,
}

impl ServerConfig {
    pub fn load(args: Args) -> anyhow::Result<Self> {
        let mut config = Self {
            addr: "0.0.0.0:8088".parse().expect("default addr"),
            db_path: PathBuf::from("codux-service.sqlite3"),
            stats_enabled: true,
            stats_path: PathBuf::from("codux-service.stats.jsonl"),
            stats_flush_interval: Duration::from_secs(10),
            pairing_ttl: Duration::from_secs(300),
            shutdown_timeout: Duration::from_secs(3),
            read_header_timeout: Duration::from_secs(10),
            config_loaded_from: None,
        };

        let config_path = args
            .config
            .clone()
            .or_else(|| env::var_os("CODEX_SERVICE_CONFIG").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));
        let config_explicit =
            args.config.is_some() || env::var_os("CODEX_SERVICE_CONFIG").is_some();
        if config_path.exists() || config_explicit {
            config.apply_file(&config_path)?;
            config.config_loaded_from = Some(config_path);
        }

        if let Some(addr) = args.addr {
            config.addr = parse_addr(&addr)?;
        }
        if let Some(path) = args.db_path {
            config.db_path = path;
        }
        if let Some(enabled) = args.stats_enabled {
            config.stats_enabled = enabled;
        }
        if let Some(path) = args.stats_path {
            config.stats_path = path;
        }
        if let Some(value) = args.stats_flush_interval_secs {
            config.stats_flush_interval = seconds("stats flush interval", value)?;
        }
        if let Some(value) = args.pairing_ttl_secs {
            config.pairing_ttl = seconds("pairing ttl", value)?;
        }
        if let Some(value) = args.shutdown_timeout_secs {
            config.shutdown_timeout = seconds("shutdown timeout", value)?;
        }
        if let Some(value) = args.read_header_timeout_secs {
            config.read_header_timeout = seconds("read header timeout", value)?;
        }

        config.validate()?;
        Ok(config)
    }

    fn apply_file(&mut self, path: &Path) -> anyhow::Result<()> {
        let text =
            fs::read_to_string(path).with_context(|| format!("read config {}", path.display()))?;
        let file: FileConfig =
            toml::from_str(&text).with_context(|| format!("parse config {}", path.display()))?;
        if let Some(server) = file.server {
            if let Some(addr) = server.addr {
                self.addr = parse_addr(&addr)?;
            }
            if let Some(value) = server.read_header_timeout_seconds {
                self.read_header_timeout = seconds("read header timeout", value)?;
            }
        }
        if let Some(database) = file.database {
            if let Some(path) = database.path {
                self.db_path = path;
            }
        }
        if let Some(stats) = file.stats {
            if let Some(enabled) = stats.enabled {
                self.stats_enabled = enabled;
            }
            if let Some(path) = stats.path {
                self.stats_path = path;
            }
            if let Some(value) = stats.flush_interval_seconds {
                self.stats_flush_interval = seconds("stats flush interval", value)?;
            }
        }
        if let Some(pairing) = file.pairing {
            if let Some(value) = pairing.ttl_seconds {
                self.pairing_ttl = seconds("pairing ttl", value)?;
            }
        }
        if let Some(shutdown) = file.shutdown {
            if let Some(value) = shutdown.timeout_seconds {
                self.shutdown_timeout = seconds("shutdown timeout", value)?;
            }
        }
        Ok(())
    }

    fn validate(&self) -> anyhow::Result<()> {
        if self.db_path.as_os_str().is_empty() {
            bail!("db path cannot be empty");
        }
        if self.stats_enabled && self.stats_path.as_os_str().is_empty() {
            bail!("stats path cannot be empty when stats are enabled");
        }
        if self.stats_enabled && self.stats_flush_interval.is_zero() {
            bail!("stats flush interval must be positive");
        }
        if self.pairing_ttl.is_zero() {
            bail!("pairing ttl must be positive");
        }
        if self.shutdown_timeout.is_zero() {
            bail!("shutdown timeout must be positive");
        }
        if self.read_header_timeout.is_zero() {
            bail!("read header timeout must be positive");
        }
        Ok(())
    }
}

fn parse_addr(value: &str) -> anyhow::Result<SocketAddr> {
    let value = if value.starts_with(':') {
        format!("0.0.0.0{value}")
    } else {
        value.to_string()
    };
    value
        .parse()
        .with_context(|| format!("invalid listen addr {value:?}"))
}

fn seconds(name: &str, value: u64) -> anyhow::Result<Duration> {
    if value == 0 {
        bail!("{name} must be positive");
    }
    Ok(Duration::from_secs(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn config_uses_defaults_without_config_file() {
        let config = ServerConfig::load(Args {
            config: Some(PathBuf::from("missing-test-config.toml")),
            addr: None,
            db_path: None,
            stats_enabled: None,
            stats_path: None,
            stats_flush_interval_secs: None,
            pairing_ttl_secs: None,
            shutdown_timeout_secs: None,
            read_header_timeout_secs: None,
        })
        .unwrap_err();
        assert!(config.to_string().contains("read config"));

        let config = ServerConfig::load(Args {
            config: None,
            addr: None,
            db_path: None,
            stats_enabled: None,
            stats_path: None,
            stats_flush_interval_secs: None,
            pairing_ttl_secs: None,
            shutdown_timeout_secs: None,
            read_header_timeout_secs: None,
        })
        .unwrap();
        assert_eq!(config.addr.port(), 8088);
        assert_eq!(config.db_path, PathBuf::from("codux-service.sqlite3"));
    }

    #[test]
    fn config_reads_toml_and_cli_overrides_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"
[server]
addr = "127.0.0.1:9000"
read_header_timeout_seconds = 11

[database]
path = "from-file.sqlite3"

[stats]
enabled = false
path = "stats.jsonl"
flush_interval_seconds = 12

[pairing]
ttl_seconds = 13

[shutdown]
timeout_seconds = 14
"#
        )
        .unwrap();

        let config = ServerConfig::load(Args {
            config: Some(path),
            addr: Some("127.0.0.1:9100".into()),
            db_path: Some(PathBuf::from("from-cli.sqlite3")),
            stats_enabled: Some(true),
            stats_path: None,
            stats_flush_interval_secs: None,
            pairing_ttl_secs: None,
            shutdown_timeout_secs: None,
            read_header_timeout_secs: None,
        })
        .unwrap();

        assert_eq!(config.addr.port(), 9100);
        assert_eq!(config.db_path, PathBuf::from("from-cli.sqlite3"));
        assert!(config.stats_enabled);
        assert_eq!(config.stats_flush_interval, Duration::from_secs(12));
        assert_eq!(config.pairing_ttl, Duration::from_secs(13));
    }

    #[test]
    fn config_accepts_go_style_port_only_addr() {
        let addr = parse_addr(":8088").unwrap();

        assert_eq!(addr, "0.0.0.0:8088".parse::<SocketAddr>().unwrap());
    }
}
