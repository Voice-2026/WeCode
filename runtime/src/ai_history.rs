mod helpers;
mod queries;
mod sessions;
mod summary;
#[cfg(test)]
mod tests;
mod types;

use rusqlite::Connection;
use std::path::PathBuf;
pub use types::*;

pub struct AIHistoryService {
    database_path: PathBuf,
}

impl AIHistoryService {
    pub fn new(support_dir: PathBuf) -> Self {
        Self {
            database_path: support_dir.join("ai-usage.sqlite3"),
        }
    }

    fn open_connection(&self) -> Result<Connection, String> {
        if !self.database_path.is_file() {
            return Err("ai-usage.sqlite3 not found".to_string());
        }
        Connection::open(&self.database_path).map_err(|error| error.to_string())
    }
}
