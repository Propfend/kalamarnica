use anyhow::Result;
use anyhow::bail;

pub fn validate_name(context_name: &str) -> Result<String> {
    if context_name.is_empty() {
        bail!("context name cannot be empty");
    }

    if !context_name
        .chars()
        .all(|character| character.is_alphanumeric() || character == '-' || character == '_')
    {
        bail!("context name must contain only alphanumeric characters, hyphens, and underscores");
    }

    Ok(context_name.to_owned())
}
