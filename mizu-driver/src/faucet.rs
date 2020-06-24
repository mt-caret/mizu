//! Parses faucet JSON files.

use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct FaucetOutput {
    pub mnemonic: Vec<String>,
    pub secret: String,
    pub amount: String,
    pub pkh: String,
    pub password: String,
    pub email: String,
}

impl FaucetOutput {
    pub fn load_from_file<P: AsRef<Path>>(
        path: P,
    ) -> Result<FaucetOutput, Box<dyn std::error::Error + Send + Sync + 'static>> {
        Ok(serde_json::from_str(&read_to_string(path)?)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_faucet_parse_succeeds() {
        let faucet = r#"{
            "mnemonic": [
              "tell",
              "alpha",
              "picnic",
              "olive",
              "fiction",
              "crop",
              "quality",
              "curtain",
              "gospel",
              "polar",
              "number",
              "journey",
              "master",
              "struggle",
              "time"
            ],
            "secret": "9eac82aba27a5ec364e2ca7f992e8b1419a7b064",
            "amount": "32518036222",
            "pkh": "tz1dYhoisPFJAD6WexiaTgjn7TBoNU6vmvac",
            "password": "va2Vuyt0A4",
            "email": "nbilkxuh.uoxwujyd@tezos.example.org"
        }"#;

        let _faucet: FaucetOutput = serde_json::from_str(faucet).unwrap();
    }
}
