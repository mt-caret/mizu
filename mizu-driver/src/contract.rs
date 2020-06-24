use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct ContractConfig {
    pub debug: bool,
    pub contract_address: String,
    pub rpc_host: String,
}

impl ContractConfig {
    pub fn load_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<ContractConfig, Box<dyn std::error::Error + Send + Sync + 'static>> {
        // Surprisingly, reading the whole file is faster.
        // https://github.com/serde-rs/json/issues/160#issuecomment-253446892
        Ok(serde_json::from_str(&read_to_string(path)?)?)
    }
}
