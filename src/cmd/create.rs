use std::env::VarError;

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use clap::Parser;

use crate::cmd::handler::Handler;
use crate::cmd::validate_name::validate_name;
use crate::context::Context;
use crate::gh_cli::GhClient;
use crate::storage::Storage;
use crate::transport::Transport;

#[derive(Parser)]
pub struct Create {
    #[arg(long, value_parser = validate_name)]
    /// Name for the new context
    name: String,

    #[arg(long)]
    /// Detect hostname and user from current gh session
    from_current: bool,

    #[arg(long)]
    /// GitHub hostname (e.g., github.com)
    hostname: Option<String>,

    #[arg(long)]
    /// GitHub username
    user: Option<String>,

    #[arg(long, default_value = "ssh")]
    /// Git transport protocol
    transport: Transport,

    #[arg(long)]
    /// SSH config host alias
    ssh_host: Option<String>,
}

impl Create {
    pub fn execute(&self, storage: &Storage) -> Result<()> {
        if storage.context_exists(&self.name) {
            bail!("context '{}' already exists", self.name);
        }

        let (hostname, user) = match self.from_current {
            true => {
                let hostname = match std::env::var("GH_HOST") {
                    Ok(env_hostname) => env_hostname,
                    Err(VarError::NotPresent) => "github.com".to_owned(),
                    Err(VarError::NotUnicode(raw_value)) => {
                        bail!("GH_HOST contains invalid unicode: {}", raw_value.display())
                    }
                };
                let user = GhClient::api_user(&hostname)?;

                (hostname, user)
            }
            false => {
                let hostname = self
                    .hostname
                    .clone()
                    .ok_or_else(|| anyhow!("--hostname is required (or use --from-current)"))?;
                let user = self
                    .user
                    .clone()
                    .ok_or_else(|| anyhow!("--user is required (or use --from-current)"))?;

                (hostname, user)
            }
        };

        let context = Context {
            hostname,
            user,
            transport: self.transport.clone(),
            ssh_host_alias: self.ssh_host.clone(),
        };

        storage.write_context(&self.name, &context)?;
        log::info!(
            "Created context '{}' ({}@{}, {})",
            self.name,
            context.user,
            context.hostname,
            context.transport
        );

        Ok(())
    }
}

impl Handler for Create {
    fn handle(&self, storage: &Storage) -> Result<()> {
        self.execute(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::Create;
    use crate::cmd::handler::Handler;
    use crate::cmd::validate_name::validate_name;
    use crate::context::Context;
    use crate::storage::Storage;
    use crate::transport::Transport;

    #[test]
    fn validate_empty_name_fails() -> Result<(), anyhow::Error> {
        let error = validate_name("").unwrap_err();
        assert_eq!(error.to_string(), "context name cannot be empty");

        Ok(())
    }

    #[test]
    fn validate_alphanumeric_name_succeeds() -> Result<(), anyhow::Error> {
        let name = "work123";
        let result = validate_name(name)?;
        assert_eq!(result, name);

        Ok(())
    }

    #[test]
    fn validate_name_with_hyphens() -> Result<(), anyhow::Error> {
        let name = "my-work";
        let result = validate_name(name)?;
        assert_eq!(result, name);

        Ok(())
    }

    #[test]
    fn validate_name_with_underscores() -> Result<(), anyhow::Error> {
        let name = "my_work";
        let result = validate_name(name)?;
        assert_eq!(result, name);

        Ok(())
    }

    #[test]
    fn validate_name_with_mixed_valid_chars() -> Result<(), anyhow::Error> {
        let name = "my-work_123";
        let result = validate_name(name)?;
        assert_eq!(result, name);

        Ok(())
    }

    #[test]
    fn validate_name_with_spaces_fails() -> Result<(), anyhow::Error> {
        let error = validate_name("my work").unwrap_err();
        assert_eq!(
            error.to_string(),
            "context name must contain only alphanumeric characters, hyphens, and underscores"
        );

        Ok(())
    }

    #[test]
    fn validate_name_with_dots_fails() -> Result<(), anyhow::Error> {
        let error = validate_name("my.work").unwrap_err();
        assert_eq!(
            error.to_string(),
            "context name must contain only alphanumeric characters, hyphens, and underscores"
        );

        Ok(())
    }

    #[test]
    fn validate_name_with_slashes_fails() -> Result<(), anyhow::Error> {
        let error = validate_name("my/work").unwrap_err();
        assert_eq!(
            error.to_string(),
            "context name must contain only alphanumeric characters, hyphens, and underscores"
        );

        Ok(())
    }

    #[test]
    fn create_context_already_exists_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        let existing_context = Context {
            hostname: "github.com".to_owned(),
            user: "existing".to_owned(),
            transport: Transport::Ssh,
            ssh_host_alias: None,
        };
        let context_name = "work";
        storage.write_context(context_name, &existing_context)?;

        let create = Create {
            name: context_name.to_owned(),
            from_current: false,
            hostname: Some("github.com".to_owned()),
            user: Some("newuser".to_owned()),
            transport: Transport::Ssh,
            ssh_host: None,
        };

        let error = create.handle(&storage).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("context '{context_name}' already exists")
        );

        Ok(())
    }

    #[test]
    fn create_context_manual_without_hostname_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let create = Create {
            name: "work".to_owned(),
            from_current: false,
            hostname: None,
            user: Some("testuser".to_owned()),
            transport: Transport::Ssh,
            ssh_host: None,
        };

        let error = create.handle(&storage).unwrap_err();
        assert_eq!(
            error.to_string(),
            "--hostname is required (or use --from-current)"
        );

        Ok(())
    }

    #[test]
    fn create_context_manual_without_user_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let create = Create {
            name: "work".to_owned(),
            from_current: false,
            hostname: Some("github.com".to_owned()),
            user: None,
            transport: Transport::Ssh,
            ssh_host: None,
        };

        let error = create.handle(&storage).unwrap_err();
        assert_eq!(
            error.to_string(),
            "--user is required (or use --from-current)"
        );

        Ok(())
    }

    #[test]
    fn create_context_manual_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let create = Create {
            name: "work".to_owned(),
            from_current: false,
            hostname: Some("github.com".to_owned()),
            user: Some("testuser".to_owned()),
            transport: Transport::Ssh,
            ssh_host: None,
        };

        create.handle(&storage)?;

        assert!(storage.context_exists("work"));
        let context = storage.read_context("work")?;
        assert_eq!(context.hostname, "github.com");
        assert_eq!(context.user, "testuser");
        assert!(matches!(context.transport, Transport::Ssh));
        assert!(context.ssh_host_alias.is_none());

        Ok(())
    }

    #[test]
    fn create_context_with_ssh_host_alias() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let create = Create {
            name: "enterprise".to_owned(),
            from_current: false,
            hostname: Some("enterprise.example.com".to_owned()),
            user: Some("admin".to_owned()),
            transport: Transport::Https,
            ssh_host: Some("gh-enterprise".to_owned()),
        };

        create.handle(&storage)?;

        let context = storage.read_context("enterprise")?;
        assert_eq!(context.ssh_host_alias.as_deref(), Some("gh-enterprise"));
        assert!(matches!(context.transport, Transport::Https));

        Ok(())
    }

    #[test]
    fn create_context_with_invalid_name_fails() -> Result<(), anyhow::Error> {
        let context_name = "invalid name!";
        let error = validate_name(context_name).unwrap_err();
        assert_eq!(
            error.to_string(),
            "context name must contain only alphanumeric characters, hyphens, and underscores"
        );

        Ok(())
    }
}
