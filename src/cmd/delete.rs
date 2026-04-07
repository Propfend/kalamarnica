use anyhow::Result;
use anyhow::bail;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::storage::Storage;

#[derive(Parser)]
pub struct Delete {
    /// Context name to delete
    name: String,
}

impl Delete {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        if !storage.context_exists(&self.name) {
            bail!("context '{}' does not exist", self.name);
        }

        storage.delete_context(&self.name)?;
        log::info!("Deleted context '{}'", self.name);

        Ok(())
    }
}

impl Handler for Delete {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::Delete;
    use crate::cmd::handler::Handler;
    use crate::context::Context;
    use crate::storage::Storage;
    use crate::transport::Transport;

    fn sample_context() -> Context {
        Context {
            hostname: "github.com".to_owned(),
            user: "testuser".to_owned(),
            transport: Transport::Ssh,
            ssh_host_alias: None,
        }
    }

    #[test]
    fn delete_nonexistent_context_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let context_name = "nonexistent";
        let handler = Delete {
            name: context_name.to_owned(),
        };

        let error = handler.handle(&storage).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("context '{context_name}' does not exist")
        );

        Ok(())
    }

    #[test]
    fn delete_existing_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;

        let handler = Delete {
            name: "work".to_owned(),
        };

        handler.handle(&storage)?;
        assert!(!storage.context_exists("work"));

        Ok(())
    }

    #[test]
    fn delete_context_with_token_removes_both() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.write_token("work", "ghp_test123")?;

        let handler = Delete {
            name: "work".to_owned(),
        };

        handler.handle(&storage)?;
        assert!(!storage.context_exists("work"));
        assert!(storage.read_token("work")?.is_none());

        Ok(())
    }
}
