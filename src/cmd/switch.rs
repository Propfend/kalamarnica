use anyhow::Result;
use anyhow::bail;
use clap::Parser;
use indoc::indoc;

use crate::cmd::handler::Handler;
use crate::gh_cli::GhCli;
use crate::storage::Storage;

#[derive(Parser)]
pub struct Switch {
    /// Context name to switch to
    name: String,
}

impl Switch {
    #[must_use]
    pub const fn for_context(name: String) -> Self {
        Self { name }
    }

    pub fn execute(&self, storage: &Storage) -> Result<()> {
        if !storage.context_exists(&self.name) {
            bail!("context '{}' does not exist", &self.name);
        }

        let context = storage.read_context(&self.name)?;
        storage.set_active(&self.name)?;

        if let Some(stored_token) = storage.read_token(&self.name)? {
            match GhCli::auth_login_with_token(&context.hostname, &stored_token) {
                Ok(()) => log::warn!("Applied stored token for '{}'", &self.name),
                Err(login_error) => {
                    log::warn!(
                        "Failed to apply stored token for '{}': {}",
                        &self.name,
                        login_error
                    );
                }
            }
        }

        match verify_auth(&context.hostname, &context.user) {
            Ok(()) => log::warn!("Authentication verified"),
            Err(verify_error) => {
                log::warn!(
                    indoc! {"
                        Authentication required for {}@{} ({})
                          Run: gh auth login --hostname {} --user {} --scopes repo,read:org
                    "},
                    context.user,
                    context.hostname,
                    verify_error,
                    context.hostname,
                    context.user
                );
            }
        }

        log::info!(
            "Switched to context '{}' ({}@{}, {})",
            &self.name,
            context.user,
            context.hostname,
            context.transport
        );

        Ok(())
    }
}

impl Handler for Switch {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

fn verify_auth(github_hostname: &str, user: &str) -> Result<()> {
    let auth_status_output = GhCli::auth_status(github_hostname)?;
    if !auth_status_output.contains(&format!("Logged in to {github_hostname} account {user}")) {
        bail!("not logged in as {user}");
    }

    GhCli::auth_switch(github_hostname, user)?;

    let authenticated_user = GhCli::api_user(github_hostname)?;
    if authenticated_user != user {
        bail!("expected user {user}, got {authenticated_user}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Switch;
    use crate::storage::Storage;

    #[test]
    fn switch_to_nonexistent_context_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let context_name = "nonexistent";
        let switch_handler = Switch::for_context(context_name.to_owned());

        let error = switch_handler.execute(&storage).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("context '{context_name}' does not exist")
        );

        Ok(())
    }
}
