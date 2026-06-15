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
    #[arg(long = "shutdown-timeout", env = "CODEX_SHUTDOWN_TIMEOUT")]
    pub shutdown_timeout_secs: Option<u64>,
    #[arg(long = "read-header-timeout", env = "CODEX_READ_HEADER_TIMEOUT")]
    pub read_header_timeout_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub addr: SocketAddr,
    pub db_path: PathBuf,
    pub shutdown_timeout: Duration,
    pub read_header_timeout: Duration,
    pub config_loaded_from: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    server: Option<FileServerConfig>,
    database: Option<FileDatabaseConfig>,
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
struct FileShutdownConfig {
    timeout_seconds: Option<u64>,
}

impl ServerConfig {
    pub fn load(args: Args) -> anyhow::Result<Self> {
        let mut config = Self {
            addr: "0.0.0.0:8088".parse().expect("default addr"),
            db_path: PathBuf::from("codux-service.sqlite3"),
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
            shutdown_timeout_secs: None,
            read_header_timeout_secs: None,
        })
        .unwrap_err();
        assert!(config.to_string().contains("read config"));

        let config = ServerConfig::load(Args {
            config: None,
            addr: None,
            db_path: None,
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

[shutdown]
timeout_seconds = 14
"#
        )
        .unwrap();

        let config = ServerConfig::load(Args {
            config: Some(path),
            addr: Some("127.0.0.1:9100".into()),
            db_path: Some(PathBuf::from("from-cli.sqlite3")),
            shutdown_timeout_secs: None,
            read_header_timeout_secs: None,
        })
        .unwrap();

        assert_eq!(config.addr.port(), 9100);
        assert_eq!(config.db_path, PathBuf::from("from-cli.sqlite3"));
        assert_eq!(config.shutdown_timeout, Duration::from_secs(14));
    }

    #[test]
    fn config_accepts_go_style_port_only_addr() {
        let addr = parse_addr(":8088").unwrap();

        assert_eq!(addr, "0.0.0.0:8088".parse::<SocketAddr>().unwrap());
    }
}
