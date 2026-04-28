use std::fs;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use octocrab::Octocrab;
use serde_yaml::Mapping;
use serde_yaml::Value;
use tokio::runtime::Runtime;

fn string_value(text: &str) -> Value {
    Value::String(text.to_owned())
}

fn github_hosts_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("could not determine config directory")?;

    Ok(config_dir.join("gh").join("hosts.yml"))
}

fn read_github_hosts() -> Result<Mapping> {
    let hosts_config_path = github_hosts_path()?;
    if !hosts_config_path.exists() {
        return Ok(Mapping::new());
    }

    let hosts_yaml_content =
        fs::read_to_string(&hosts_config_path).context("could not read gh hosts config")?;
    if hosts_yaml_content.trim().is_empty() {
        return Ok(Mapping::new());
    }

    let parsed_value: Value =
        serde_yaml::from_str(&hosts_yaml_content).context("could not parse gh hosts config")?;

    match parsed_value {
        Value::Mapping(hosts_mapping) => Ok(hosts_mapping),
        _ => bail!("gh hosts config is not a YAML mapping"),
    }
}

fn write_github_hosts(hosts_config: &Mapping) -> Result<()> {
    let hosts_config_path = github_hosts_path()?;

    if let Some(parent_dir) = hosts_config_path.parent() {
        fs::create_dir_all(parent_dir).context("could not create gh config directory")?;
    }

    let hosts_yaml_content = serde_yaml::to_string(&Value::Mapping(hosts_config.clone()))
        .context("could not serialize gh hosts config")?;

    fs::write(&hosts_config_path, hosts_yaml_content).context("could not write gh hosts config")
}

fn fetch_api_user(hostname: &str, token: &str) -> Result<String> {
    let async_runtime = Runtime::new().context("could not create async runtime")?;

    async_runtime.block_on(async {
        let mut octocrab_builder = Octocrab::builder().personal_token(token.to_owned());
        if hostname != "github.com" {
            octocrab_builder = octocrab_builder.base_uri(format!("https://{hostname}/api/v3"))?;
        }
        let github_client = octocrab_builder.build()?;
        let current_user = github_client.current().user().await?;

        Ok(current_user.login)
    })
}

fn ensure_host_entry<'hosts_config>(
    hosts_config: &'hosts_config mut Mapping,
    hostname: &str,
) -> Result<&'hosts_config mut Mapping> {
    let hostname_key = string_value(hostname);
    if !hosts_config.contains_key(&hostname_key) {
        hosts_config.insert(hostname_key.clone(), Value::Mapping(Mapping::new()));
    }

    hosts_config
        .get_mut(&hostname_key)
        .and_then(|host_value| host_value.as_mapping_mut())
        .context("host entry is not a mapping")
}

fn ensure_users_map(host_mapping: &mut Mapping) -> Result<&mut Mapping> {
    let users_key = string_value("users");
    if !host_mapping.contains_key(&users_key) {
        host_mapping.insert(users_key.clone(), Value::Mapping(Mapping::new()));
    }

    host_mapping
        .get_mut(&users_key)
        .and_then(|users_value| users_value.as_mapping_mut())
        .context("users entry is not a mapping")
}

pub struct GhClient;

impl GhClient {
    pub fn auth_login_with_token(github_hostname: &str, token: &str) -> Result<()> {
        let current_user = fetch_api_user(github_hostname, token)?;

        let mut hosts_config = read_github_hosts()?;
        let host_mapping = ensure_host_entry(&mut hosts_config, github_hostname)?;

        host_mapping.insert(string_value("user"), string_value(&current_user));
        host_mapping.insert(string_value("oauth_token"), string_value(token));

        let users_mapping = ensure_users_map(host_mapping)?;
        let mut user_entry = Mapping::new();
        user_entry.insert(string_value("oauth_token"), string_value(token));
        users_mapping.insert(string_value(&current_user), Value::Mapping(user_entry));

        write_github_hosts(&hosts_config)?;

        Ok(())
    }

    pub fn auth_status(github_hostname: &str) -> Result<String> {
        let hosts_config = read_github_hosts()?;

        match hosts_config.get(string_value(github_hostname)) {
            Some(Value::Mapping(host_mapping)) => {
                let current_user = host_mapping
                    .get(string_value("user"))
                    .and_then(|user_value| user_value.as_str())
                    .unwrap_or("unknown");

                Ok(format!(
                    "Logged in to {github_hostname} account {current_user}"
                ))
            }
            _ => Ok(format!(
                "You are not logged into any GitHub hosts on {github_hostname}"
            )),
        }
    }

    pub fn auth_switch(github_hostname: &str, user: &str) -> Result<()> {
        let mut hosts_config = read_github_hosts()?;

        let host_mapping = hosts_config
            .get_mut(string_value(github_hostname))
            .and_then(|host_value| host_value.as_mapping_mut())
            .ok_or_else(|| anyhow!("not logged in to {github_hostname}"))?;

        let users_mapping = host_mapping
            .get(string_value("users"))
            .and_then(|users_value| users_value.as_mapping())
            .ok_or_else(|| anyhow!("no users configured for {github_hostname}"))?;

        if !users_mapping.contains_key(string_value(user)) {
            bail!("account {user} not found on {github_hostname}");
        }

        let user_token = users_mapping
            .get(string_value(user))
            .and_then(|user_value| user_value.as_mapping())
            .and_then(|user_entry| user_entry.get(string_value("oauth_token")))
            .and_then(|token_value| token_value.as_str())
            .map(ToOwned::to_owned);

        host_mapping.insert(string_value("user"), string_value(user));

        if let Some(stored_token) = user_token {
            host_mapping.insert(string_value("oauth_token"), string_value(&stored_token));
        }

        write_github_hosts(&hosts_config)?;

        Ok(())
    }

    pub fn api_user(hostname: &str) -> Result<String> {
        let hosts_config = read_github_hosts()?;

        let host_mapping = hosts_config
            .get(string_value(hostname))
            .and_then(|host_value| host_value.as_mapping())
            .ok_or_else(|| anyhow!("not logged in to {hostname}"))?;

        let oauth_token = host_mapping
            .get(string_value("oauth_token"))
            .and_then(|token_value| token_value.as_str())
            .ok_or_else(|| {
                anyhow!("no token found for {hostname} (token may be in system keyring)")
            })?;

        fetch_api_user(hostname, oauth_token)
    }
}

#[cfg(test)]
mod tests {
    use serde_yaml::Mapping;
    use serde_yaml::Value;

    use super::ensure_host_entry;
    use super::ensure_users_map;
    use super::string_value;

    #[test]
    fn host_entry_creates_new_entry() -> Result<(), anyhow::Error> {
        let mut config = Mapping::new();
        let entry = ensure_host_entry(&mut config, "github.com")?;
        assert!(entry.is_empty());
        assert!(config.contains_key(string_value("github.com")));

        Ok(())
    }

    #[test]
    fn host_entry_returns_existing_entry() -> Result<(), anyhow::Error> {
        let mut config = Mapping::new();
        let mut existing = Mapping::new();
        existing.insert(string_value("user"), string_value("octocat"));
        config.insert(string_value("github.com"), Value::Mapping(existing));

        let entry = ensure_host_entry(&mut config, "github.com")?;
        assert_eq!(
            entry.get(string_value("user")),
            Some(&string_value("octocat"))
        );

        Ok(())
    }

    #[test]
    fn host_entry_does_not_overwrite_existing() -> Result<(), anyhow::Error> {
        let mut config = Mapping::new();
        let mut existing = Mapping::new();
        existing.insert(string_value("user"), string_value("octocat"));
        existing.insert(string_value("oauth_token"), string_value("ghp_abc"));
        config.insert(string_value("github.com"), Value::Mapping(existing));

        let entry = ensure_host_entry(&mut config, "github.com")?;
        assert_eq!(entry.len(), 2);

        Ok(())
    }

    #[test]
    fn users_map_creates_new_entry() -> Result<(), anyhow::Error> {
        let mut host = Mapping::new();
        let users = ensure_users_map(&mut host)?;
        assert!(users.is_empty());
        assert!(host.contains_key(string_value("users")));

        Ok(())
    }

    #[test]
    fn users_map_returns_existing_entry() -> Result<(), anyhow::Error> {
        let mut host = Mapping::new();
        let mut users = Mapping::new();
        users.insert(string_value("octocat"), Value::Mapping(Mapping::new()));
        host.insert(string_value("users"), Value::Mapping(users));

        let users_ref = ensure_users_map(&mut host)?;
        assert!(users_ref.contains_key(string_value("octocat")));

        Ok(())
    }

    #[test]
    fn ensure_users_map_does_not_overwrite_existing() -> Result<(), anyhow::Error> {
        let mut host = Mapping::new();
        let mut users = Mapping::new();
        users.insert(string_value("user1"), Value::Mapping(Mapping::new()));
        users.insert(string_value("user2"), Value::Mapping(Mapping::new()));
        host.insert(string_value("users"), Value::Mapping(users));

        let users_ref = ensure_users_map(&mut host)?;
        assert_eq!(users_ref.len(), 2);

        Ok(())
    }
}
