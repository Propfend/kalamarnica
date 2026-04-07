use std::fmt::Write as _;

use anyhow::Result;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::storage::Storage;

#[derive(Parser)]
pub struct List;

impl List {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        let context_names = storage.list_context_names()?;
        let active_context_name = storage.get_active()?;

        if context_names.is_empty() {
            log::info!(
                "No contexts found. Create one with: kalamarnica create --name <name> --hostname <host> --user <user>"
            );

            return Ok(());
        }

        for context_name in &context_names {
            let context = storage.read_context(context_name)?;
            let active_marker = match active_context_name.as_deref() == Some(context_name.as_str())
            {
                true => " *",
                false => "",
            };

            let mut context_summary = format!(
                "{}@{}, {}",
                context.user, context.hostname, context.transport
            );
            if let Some(ssh_host_alias) = &context.ssh_host_alias {
                write!(context_summary, ", ssh_host={ssh_host_alias}")?;
            }

            log::info!("{context_name}{active_marker} ({context_summary})");
        }

        Ok(())
    }
}

impl Handler for List {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::List;
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
    fn list_empty_contexts_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        let handler = List;

        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn list_single_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;

        let handler = List;
        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn list_multiple_contexts_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.write_context("personal", &sample_context())?;

        let handler = List;
        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn list_with_active_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.write_context("personal", &sample_context())?;
        storage.set_active("work")?;

        let handler = List;
        handler.handle(&storage)?;

        Ok(())
    }

    #[test]
    fn list_context_with_ssh_alias_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        let context = Context {
            hostname: "github.com".to_owned(),
            user: "testuser".to_owned(),
            transport: Transport::Ssh,
            ssh_host_alias: Some("gh-work".to_owned()),
        };
        storage.write_context("work", &context)?;

        let handler = List;
        handler.handle(&storage)?;

        Ok(())
    }
}
