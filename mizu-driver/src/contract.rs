use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ContractConfig {
    pub debug: bool,
    pub contract_address: String,
    pub rpc_host: String,
}
