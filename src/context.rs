use serde::Deserialize;
use serde::Serialize;

use crate::transport::Transport;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Context {
    pub hostname: String,
    pub user: String,
    pub transport: Transport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_host_alias: Option<String>,
}
