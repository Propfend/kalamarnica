use anyhow::Result;

use crate::storage::Storage;

pub trait Handler {
    fn handle(&self, storage: &Storage) -> Result<()>;
}
