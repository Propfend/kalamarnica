use std::fs;
use std::path::Path;

use anyhow::Result;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::repo_root;
use crate::storage::Storage;

#[derive(Parser)]
pub struct Current;

impl Current {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        let active_context_name = storage.get_active()?;

        match &active_context_name {
            Some(context_name) => {
                let context = storage.read_context(context_name)?;
                log::info!(
                    "{context_name} ({}@{}, {})",
                    context.user,
                    context.hostname,
                    context.transport
                );
            }
            None => log::info!("No active context"),
        }

        if let Some(repo_root_path) = repo_root::repo_root()? {
            let binding_path = Path::new(&repo_root_path).join(".ghcontext");
            if binding_path.exists() {
                let bound_context_name = fs::read_to_string(&binding_path)?.trim().to_owned();
                log::info!("Repo-bound context: {bound_context_name}");
            }
        }

        Ok(())
    }
}

impl Handler for Current {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::Current;
    use crate::cmd::handler::Handler;
    use crate::context::Context;
    use crate::storage::Storage;
    use crate::test_utils::CWD_MUTEX;
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
    fn no_active_context_succeeds() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir)?;
        git2::Repository::init(&repo_dir)?;
        std::env::set_current_dir(&repo_dir)?;

        let handler = Current;
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        Ok(())
    }

    #[test]
    fn with_active_context_succeeds() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.set_active("work")?;

        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir)?;
        git2::Repository::init(&repo_dir)?;
        std::env::set_current_dir(&repo_dir)?;

        let handler = Current;
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        Ok(())
    }

    #[test]
    fn with_repo_bound_context_succeeds() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir)?;
        git2::Repository::init(&repo_dir)?;
        std::fs::write(repo_dir.join(".ghcontext"), "work")?;
        std::env::set_current_dir(&repo_dir)?;

        let handler = Current;
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        Ok(())
    }

    #[test]
    fn outside_git_repo_succeeds() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        std::env::set_current_dir(tmp.path())?;

        let handler = Current;
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        Ok(())
    }
}
