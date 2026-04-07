use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;

use crate::context::Context;

pub struct Storage {
    base_dir: PathBuf,
    contexts_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let base_dir = dirs::config_dir()
            .context("could not determine config directory")?
            .join("kalamarnica");
        let contexts_dir = base_dir.join("contexts");

        fs::create_dir_all(&contexts_dir).context("could not create contexts directory")?;

        Ok(Self {
            base_dir,
            contexts_dir,
        })
    }

    #[must_use]
    pub fn context_folder_path(&self, context_name: &str) -> PathBuf {
        self.contexts_dir.join(context_name)
    }

    #[must_use]
    pub fn context_exists(&self, context_name: &str) -> bool {
        self.context_folder_path(context_name).exists()
    }

    pub fn read_context(&self, context_name: &str) -> Result<Context> {
        let context_file_path = self
            .context_folder_path(context_name)
            .join("configuration.toml");
        let serialized_context = fs::read_to_string(&context_file_path)
            .with_context(|| format!("could not read context '{context_name}'"))?;

        toml::from_str(&serialized_context)
            .with_context(|| format!("could not parse context '{context_name}'"))
    }

    pub fn write_context(&self, context_name: &str, context: &Context) -> Result<()> {
        let context_folder_path = self.context_folder_path(context_name);

        let context_file_path = context_folder_path.join("configuration.toml");

        let serialized_context =
            toml::to_string_pretty(context).context("could not serialize context")?;

        fs::create_dir_all(context_folder_path).context("could not create context directory")?;

        fs::write(&context_file_path, serialized_context)
            .with_context(|| format!("could not write context '{context_name}'"))
    }

    pub fn delete_context(&self, context_name: &str) -> Result<()> {
        let context_folder_path = self.context_folder_path(context_name);
        if context_folder_path.exists() {
            fs::remove_dir_all(&context_folder_path)
                .with_context(|| format!("could not delete context '{context_name}'"))?;
        }

        if self.get_active()?.as_deref() == Some(context_name) {
            let active_file_path = self.active_path();
            if active_file_path.exists() {
                fs::remove_file(&active_file_path).context("could not clear active context")?;
            }
        }

        Ok(())
    }

    pub fn list_context_names(&self) -> Result<Vec<String>> {
        let mut context_names = Vec::new();

        for dir_entry_result in
            fs::read_dir(&self.contexts_dir).context("could not read contexts directory")?
        {
            let dir_entry = dir_entry_result?;
            let entry_path = dir_entry.path();

            if entry_path.is_dir()
                && entry_path.join("configuration.toml").exists()
                && let Some(folder_name) = entry_path
                    .file_name()
                    .and_then(|folder_name| folder_name.to_str())
            {
                context_names.push(folder_name.to_owned());
            }
        }

        context_names.sort();

        Ok(context_names)
    }

    pub fn read_token(&self, context_name: &str) -> Result<Option<String>> {
        let token_file_path = self.context_folder_path(context_name).join("token");
        if !token_file_path.exists() {
            return Ok(None);
        }

        let token = fs::read_to_string(&token_file_path)
            .with_context(|| format!("could not read token for '{context_name}'"))?;

        Ok(Some(token))
    }

    pub fn write_token(&self, context_name: &str, token: &str) -> Result<()> {
        let token_file_path = self.context_folder_path(context_name).join("token");

        fs::write(&token_file_path, token)
            .with_context(|| format!("could not write context '{context_name}'"))?;

        fs::set_permissions(&token_file_path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("could not set permissions on token for '{context_name}'"))?;

        Ok(())
    }

    pub fn delete_token(&self, context_name: &str) -> Result<()> {
        let token_file_path = self.context_folder_path(context_name).join("token");
        if token_file_path.exists() {
            fs::remove_file(&token_file_path)
                .with_context(|| format!("could not delete token for '{context_name}'"))?;
        }

        Ok(())
    }

    pub fn get_active(&self) -> Result<Option<String>> {
        let active_file_path = self.active_path();
        if !active_file_path.exists() {
            return Ok(None);
        }

        let active_context_name = fs::read_to_string(&active_file_path)
            .context("could not read active context")?
            .trim()
            .to_owned();

        match active_context_name.is_empty() {
            true => Ok(None),
            false => Ok(Some(active_context_name)),
        }
    }

    pub fn set_active(&self, context_name: &str) -> Result<()> {
        fs::write(self.active_path(), context_name).context("could not set active context")
    }

    fn active_path(&self) -> PathBuf {
        self.base_dir.join("active")
    }
}

#[cfg(test)]
impl Storage {
    pub fn with_base_dir(base_dir: PathBuf) -> Result<Self> {
        let contexts_dir = base_dir.join("contexts");
        fs::create_dir_all(&contexts_dir).context("could not create test contexts directory")?;

        Ok(Self {
            base_dir,
            contexts_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt as _;

    use super::Storage;
    use crate::context::Context;
    use crate::transport::Transport;

    fn sample_context() -> Context {
        Context {
            hostname: "github.com".to_owned(),
            user: "testuser".to_owned(),
            transport: Transport::Ssh,
            ssh_host_alias: None,
        }
    }

    fn sample_context_with_ssh_host_alias() -> Context {
        Context {
            hostname: "enterprise.example.com".to_owned(),
            user: "admin".to_owned(),
            transport: Transport::Https,
            ssh_host_alias: Some("gh-enterprise".to_owned()),
        }
    }

    #[test]
    fn context_folder_path_is_named_after_context() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        let path = storage.context_folder_path("mycontext");
        assert_eq!(path, tmp.path().join("contexts").join("mycontext"));

        Ok(())
    }

    #[test]
    fn write_context_creates_configuration_toml_inside_folder() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;

        let config_path = storage
            .context_folder_path("work")
            .join("configuration.toml");
        assert!(config_path.exists());

        Ok(())
    }

    #[test]
    fn write_token_creates_token_file_inside_folder() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.write_token("work", "ghp_test123")?;

        let token_path = storage.context_folder_path("work").join("token");
        assert!(token_path.exists());

        Ok(())
    }

    #[test]
    fn context_does_not_exist_initially() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        assert!(!storage.context_exists("nonexistent"));

        Ok(())
    }

    #[test]
    fn write_and_read_context_roundtrip() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        let context = sample_context();

        storage.write_context("work", &context)?;
        assert!(storage.context_exists("work"));

        let loaded = storage.read_context("work")?;
        assert_eq!(loaded.hostname, "github.com");
        assert_eq!(loaded.user, "testuser");
        assert!(matches!(loaded.transport, Transport::Ssh));
        assert!(loaded.ssh_host_alias.is_none());

        Ok(())
    }

    #[test]
    fn write_and_read_context_with_ssh_alias() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        let context = sample_context_with_ssh_host_alias();

        storage.write_context("enterprise", &context)?;
        let loaded = storage.read_context("enterprise")?;
        assert_eq!(loaded.hostname, "enterprise.example.com");
        assert_eq!(loaded.user, "admin");
        assert!(matches!(loaded.transport, Transport::Https));
        assert_eq!(loaded.ssh_host_alias.as_deref(), Some("gh-enterprise"));

        Ok(())
    }

    #[test]
    fn read_nonexistent_context_fails() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;

        let context_name = "missing";
        let error = storage.read_context(context_name).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("could not read context '{context_name}'")
        );

        Ok(())
    }

    #[test]
    fn delete_context_removes_file() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;

        storage.delete_context("work")?;
        assert!(!storage.context_exists("work"));

        Ok(())
    }

    #[test]
    fn delete_context_removes_entire_folder_including_token() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.write_token("work", "ghp_test123")?;

        storage.delete_context("work")?;
        assert!(!storage.context_folder_path("work").exists());

        Ok(())
    }

    #[test]
    fn delete_active_context_clears_active() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.set_active("work")?;

        storage.delete_context("work")?;
        assert!(storage.get_active()?.is_none());

        Ok(())
    }

    #[test]
    fn delete_non_active_context_preserves_active() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.write_context("personal", &sample_context())?;
        storage.set_active("personal")?;

        storage.delete_context("work")?;
        assert_eq!(storage.get_active()?.as_deref(), Some("personal"));

        Ok(())
    }

    #[test]
    fn delete_nonexistent_context_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.delete_context("nonexistent")?;

        Ok(())
    }

    #[test]
    fn list_empty_contexts() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        let names = storage.list_context_names()?;
        assert!(names.is_empty());

        Ok(())
    }

    #[test]
    fn list_context_names_sorted_alphabetically() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("zebra", &sample_context())?;
        storage.write_context("alpha", &sample_context())?;
        storage.write_context("middle", &sample_context())?;

        let names = storage.list_context_names()?;
        assert_eq!(names, vec!["alpha", "middle", "zebra"]);

        Ok(())
    }

    #[test]
    fn list_context_names_ignores_stray_files_in_contexts_dir() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;

        std::fs::write(tmp.path().join("contexts").join("stray_file.txt"), "junk")?;

        let names = storage.list_context_names()?;
        assert_eq!(names, vec!["work"]);

        Ok(())
    }

    #[test]
    fn list_context_names_ignores_folders_without_configuration() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;

        std::fs::create_dir_all(tmp.path().join("contexts").join("empty_folder"))?;

        let names = storage.list_context_names()?;
        assert_eq!(names, vec!["work"]);

        Ok(())
    }

    #[test]
    fn read_token_returns_none_when_not_stored() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        assert!(storage.read_token("work")?.is_none());

        Ok(())
    }

    #[test]
    fn write_and_read_token_roundtrip() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.write_token("work", "ghp_secret123")?;

        let token = storage.read_token("work")?;
        assert_eq!(token.as_deref(), Some("ghp_secret123"));

        Ok(())
    }

    #[test]
    fn token_file_has_restricted_permissions() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.write_token("work", "ghp_secret123")?;

        let token_path = storage.context_folder_path("work").join("token");
        let metadata = std::fs::metadata(token_path)?;
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);

        Ok(())
    }

    #[test]
    fn delete_existing_token() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.write_token("work", "ghp_test")?;

        storage.delete_token("work")?;
        assert!(storage.read_token("work")?.is_none());

        Ok(())
    }

    #[test]
    fn delete_token_preserves_context_configuration() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.write_context("work", &sample_context())?;
        storage.write_token("work", "ghp_test")?;

        storage.delete_token("work")?;
        assert!(storage.context_exists("work"));

        let context = storage.read_context("work")?;
        assert_eq!(context.hostname, "github.com");

        Ok(())
    }

    #[test]
    fn delete_nonexistent_token_succeeds() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.delete_token("nonexistent")?;

        Ok(())
    }

    #[test]
    fn get_active_returns_none_initially() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        assert!(storage.get_active()?.is_none());

        Ok(())
    }

    #[test]
    fn set_and_get_active_roundtrip() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        storage.set_active("work")?;

        assert_eq!(storage.get_active()?.as_deref(), Some("work"));

        Ok(())
    }

    #[test]
    fn get_active_returns_none_for_empty_file() -> Result<(), anyhow::Error> {
        let tmp = tempfile::tempdir()?;
        let storage = Storage::with_base_dir(tmp.path().to_path_buf())?;
        std::fs::write(tmp.path().join("active"), "")?;

        assert!(storage.get_active()?.is_none());

        Ok(())
    }
}
