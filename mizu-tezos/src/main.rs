use num_bigint::{BigInt, BigUint};
use serde::Deserialize;
use std::io;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
enum TezosError {
    #[error("failed to parse url: {0}")]
    UrlParse(url::ParseError),
    #[error("deserialization error: {0}")]
    Deserialize(io::Error),
}

#[derive(Deserialize, Debug)]
struct Bootstrapped {
    block: String,
    timestamp: String,
}

fn bootstrapped(host: &Url) -> Result<Bootstrapped, TezosError> {
    let url = host
        .join("monitor/bootstrapped")
        .map_err(TezosError::UrlParse)?;

    let resp = ureq::get(url.as_str()).call();
    resp.into_json_deserialize()
        .map_err(TezosError::Deserialize)
}

#[derive(Deserialize, Debug)]
struct Constants {
    proof_of_work_nonce_size: u8,
    nonce_length: u8,
    max_revelations_per_block: u8,
    max_operation_data_length: i32,
    max_proposals_per_delegate: u8,
    preserved_cycles: u8,
    blocks_per_cycle: i32,
    blocks_per_commitment: i32,
    blocks_per_roll_snapshot: i32,
    blocks_per_voting_period: i32,
    #[serde(with = "serde_with::rust::seq_display_fromstr")]
    time_between_blocks: Vec<i64>,
    endorsers_per_block: u16,
    #[serde(with = "serde_with::rust::display_fromstr")]
    hard_gas_limit_per_operation: BigInt,
    #[serde(with = "serde_with::rust::display_fromstr")]
    hard_gas_limit_per_block: BigInt,
    #[serde(with = "serde_with::rust::display_fromstr")]
    proof_of_work_threshold: i64,
    #[serde(with = "serde_with::rust::display_fromstr")]
    tokens_per_roll: BigUint,
    michelson_maximum_type_size: u16,
    #[serde(with = "serde_with::rust::display_fromstr")]
    seed_nonce_revelation_tip: BigUint,
    origination_size: i32,
    #[serde(with = "serde_with::rust::display_fromstr")]
    block_security_deposit: BigUint,
    #[serde(with = "serde_with::rust::display_fromstr")]
    endorsement_security_deposit: BigUint,
    #[serde(with = "serde_with::rust::seq_display_fromstr")]
    baking_reward_per_endorsement: Vec<BigUint>,
    #[serde(with = "serde_with::rust::seq_display_fromstr")]
    endorsement_reward: Vec<BigUint>,
    #[serde(with = "serde_with::rust::display_fromstr")]
    cost_per_byte: BigUint,
    #[serde(with = "serde_with::rust::display_fromstr")]
    hard_storage_limit_per_operation: BigInt,
    #[serde(with = "serde_with::rust::display_fromstr")]
    test_chain_duration: i64,
    quorum_min: i32,
    quorum_max: i32,
    min_proposal_quorum: i32,
    initial_endorsers: u16,
    #[serde(with = "serde_with::rust::display_fromstr")]
    delay_per_missing_endorsement: i64,
}

fn constants(host: &Url) -> Result<Constants, TezosError> {
    let url = host
        .join("chains/main/blocks/head/context/constants")
        .map_err(TezosError::UrlParse)?;

    let resp = ureq::get(url.as_str()).call();
    resp.into_json_deserialize()
        .map_err(TezosError::Deserialize)
}

fn main() -> Result<(), TezosError> {
    let node_host: Url =
        Url::parse("https://carthagenet.smartpy.io").map_err(TezosError::UrlParse)?;

    let bootstrapped = bootstrapped(&node_host)?;

    println!("bootstrapped: {:?}", bootstrapped);

    let constants = constants(&node_host)?;

    println!("constants: {:?}", constants);

    Ok(())
}
