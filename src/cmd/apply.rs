use std::fs;
use std::path::Path;

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::cmd::switch::Switch;
use crate::repo_root;
use crate::storage::Storage;

#[derive(Parser)]
pub struct Apply;

impl Apply {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        let repo_root_path =
            repo_root::repo_root()?.ok_or_else(|| anyhow!("not inside a git repository"))?;

        let binding_path = Path::new(&repo_root_path).join(".ghcontext");
        if !binding_path.exists() {
            bail!("no context bound to this repository (use 'kalamarnica bind <name>' first)");
        }

        let context_name = fs::read_to_string(&binding_path)?.trim().to_owned();
        let switch = Switch::for_context(context_name);

        switch.execute(storage)
    }
}

impl Handler for Apply {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::Apply;
    use crate::cmd::handler::Handler;
    use crate::storage::Storage;
    use crate::test_utils::CWD_MUTEX;

    #[test]
    fn apply_outside_git_repo_fails() -> Result<(), anyhow::Error> {
        let _guard = CWD_MUTEX
            .lock()
            .map_err(|poison_error| anyhow::anyhow!("{poison_error}"))?;
        let original_cwd = std::env::current_dir()?;

        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        std::env::set_current_dir(tmp.path())?;

        let handler = Apply;
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        let error = result.unwrap_err();
        assert_eq!(error.to_string(), "not inside a git repository");

        Ok(())
    }

    #[test]
    fn apply_without_ghcontext_file_fails() -> Result<(), anyhow::Error> {
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

        let handler = Apply;
        let result = handler.handle(&storage);

        std::env::set_current_dir(&original_cwd)?;
        let error = result.unwrap_err();
        assert_eq!(
            error.to_string(),
            "no context bound to this repository (use 'kalamarnica bind <name>' first)"
        );

        Ok(())
    }
}
