use std::fmt;
use std::str::FromStr;

use anyhow::bail;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Ssh,
    Https,
}

impl fmt::Display for Transport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ssh => write!(formatter, "ssh"),
            Self::Https => write!(formatter, "https"),
        }
    }
}

impl FromStr for Transport {
    type Err = anyhow::Error;

    fn from_str(transport_text: &str) -> Result<Self, Self::Err> {
        match transport_text.to_lowercase().as_str() {
            "ssh" => Ok(Self::Ssh),
            "https" => Ok(Self::Https),
            unknown_transport => {
                bail!("invalid transport: {unknown_transport} (expected 'ssh' or 'https')")
            }
        }
    }
}
