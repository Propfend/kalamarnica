use std::fs;
use std::path::Path;

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::repo_root;
use crate::storage::Storage;

#[derive(Parser)]
pub struct Bind {
    /// Context name to bind to this repository
    name: String,
}

impl Bind {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        if !storage.context_exists(&self.name) {
            bail!("context '{}' does not exist", self.name);
        }

        let repo_root_path =
            repo_root::repo_root()?.ok_or_else(|| anyhow!("not inside a git repository"))?;

        let binding_path = Path::new(&repo_root_path).join(".ghcontext");
        fs::write(&binding_path, &self.name)?;
        log::info!("Bound context '{}' to {repo_root_path}", self.name);

        Ok(())
    }
}

impl Handler for Bind {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::Bind;
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
    fn bind_nonexistent_context_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let context_name = "nonexistent";
        let handler = Bind {
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
    fn bind_outside_git_repo_fails() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;

        let non_git_dir = tmp.path().join("not-a-repo");
        std::fs::create_dir_all(&non_git_dir)?;
        std::env::set_current_dir(&non_git_dir)?;

        let handler = Bind {
            name: "work".to_owned(),
        };
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        let error = result.unwrap_err();
        assert_eq!(error.to_string(), "not inside a git repository");

        Ok(())
    }

    #[test]
    fn bind_inside_git_repo_creates_ghcontext_file() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;

        let repo_dir = tmp.path().join("repo");
        std::fs::create_dir_all(&repo_dir)?;
        git2::Repository::init(&repo_dir)?;
        std::env::set_current_dir(&repo_dir)?;

        let handler = Bind {
            name: "work".to_owned(),
        };
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        result?;

        let ghcontext_content = std::fs::read_to_string(repo_dir.join(".ghcontext"))?;
        assert_eq!(ghcontext_content, "work");

        Ok(())
    }
}
