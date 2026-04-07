use anyhow::Result;
use clap::Parser;
use indoc::formatdoc;

use crate::cmd::handler::Handler;
use crate::gh_cli::GhCli;
use crate::storage::Storage;

#[derive(Parser)]
pub struct AuthStatus;

impl AuthStatus {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        let context_names = storage.list_context_names()?;
        let active_context_name = storage.get_active()?;

        if context_names.is_empty() {
            log::info!("No contexts found.");

            return Ok(());
        }

        for context_name in &context_names {
            let context = storage.read_context(context_name)?;

            let active_marker = match active_context_name.as_deref() == Some(context_name.as_str())
            {
                true => " *",
                false => "",
            };
            let has_stored_token = storage.read_token(context_name)?.is_some();
            let token_info = match has_stored_token {
                true => "stored",
                false => "none (using shared keyring)",
            };

            let is_authenticated = GhCli::auth_status(&context.hostname)
                .map(|auth_status_output| {
                    auth_status_output.contains(&format!(
                        "Logged in to {} account {}",
                        context.hostname, context.user
                    ))
                })
                .unwrap_or(false);

            let auth_info = match is_authenticated {
                true => "verified".to_owned(),
                false => formatdoc! {"
                    not authenticated
                        Run: gh auth login --hostname {} --user {} --scopes repo,read:org",
                    context.hostname,
                    context.user,
                },
            };

            let status_entry = formatdoc! {"
                {context_name}{active_marker}
                  Host: {hostname}
                  User: {user}
                  Transport: {transport}
                  Token: {token_info}
                  Auth: {auth_info}",
                hostname = context.hostname,
                user = context.user,
                transport = context.transport,
            };

            log::info!("{status_entry}");
        }

        Ok(())
    }
}

impl Handler for AuthStatus {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::AuthStatus;
    use crate::cmd::handler::Handler;
    use crate::storage::Storage;

    #[test]
    fn auth_status_with_no_contexts_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let handler = AuthStatus;
        handler.handle(&storage)?;

        Ok(())
    }
}
