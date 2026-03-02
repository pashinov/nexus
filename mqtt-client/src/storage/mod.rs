use std::path::Path;

pub use self::config::StorageConfig;

mod config;

use anyhow::Context;
use redb::{Database, TableDefinition};

const CONFIG_TABLE: TableDefinition<&str, &str> = TableDefinition::new("config");

pub struct Storage {
    db: Database,
}

impl Storage {
    pub fn open(config: &StorageConfig) -> anyhow::Result<Self> {
        Self::open_path(&config.db_path)
    }

    fn open_path(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("failed to create storage directory")?;
        }
        let db = Database::create(path).context("failed to open storage")?;
        Ok(Self { db })
    }

    pub fn client_id(&self) -> anyhow::Result<uuid::Uuid> {
        let read_txn = self.db.begin_read()?;

        if let Ok(table) = read_txn.open_table(CONFIG_TABLE)
            && let Some(value) = table.get("client_id")?
        {
            let id = value.value().parse().context("invalid client_id in storage")?;
            return Ok(id);
        }

        let id = uuid::Uuid::new_v4();
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(CONFIG_TABLE)?;
            table.insert("client_id", id.to_string().as_str())?;
        }
        write_txn.commit()?;

        tracing::info!(%id, "generated new client_id");

        Ok(id)
    }
}
