mod crypto;
mod helper;
mod michelson;

use michelson::Expr;
use num_bigint::{BigInt, BigUint};
use serde::Deserialize;
use serde_json::Value;
use std::io;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
enum TezosError {
    #[error("failed to parse url: {0}")]
    UrlParse(url::ParseError),
    #[error("deserialization error: {0}")]
    Deserialize(io::Error),
    #[error("deserialization error: {0}")]
    DeserializeBigInt(num_bigint::ParseBigIntError),
    #[error("crypto error: {0}")]
    Crypto(crypto::Error),
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

    ureq::get(url.as_str())
        .call()
        .into_json_deserialize()
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
    #[serde(with = "helper::seq_display_fromstr")]
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
    #[serde(with = "helper::seq_display_fromstr")]
    baking_reward_per_endorsement: Vec<BigUint>,
    #[serde(with = "helper::seq_display_fromstr")]
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

    ureq::get(url.as_str())
        .call()
        .into_json_deserialize()
        .map_err(TezosError::Deserialize)
}

fn head_hash(host: &Url) -> Result<String, TezosError> {
    let url = host
        .join("chains/main/blocks/head/hash")
        .map_err(TezosError::UrlParse)?;

    ureq::get(url.as_str())
        .call()
        .into_json_deserialize()
        .map_err(TezosError::Deserialize)
}

fn chain_id(host: &Url) -> Result<String, TezosError> {
    let url = host
        .join("chains/main/chain_id")
        .map_err(TezosError::UrlParse)?;

    ureq::get(url.as_str())
        .call()
        .into_json_deserialize()
        .map_err(TezosError::Deserialize)
}

fn counter(host: &Url, address: &str) -> Result<BigInt, TezosError> {
    let url = host
        .join(
            &[
                "chains/main/blocks/head/context/contracts/",
                address,
                "/counter",
            ]
            .concat(),
        )
        .map_err(TezosError::UrlParse)?;

    let s: String = ureq::get(url.as_str())
        .call()
        .into_json_deserialize()
        .map_err(TezosError::Deserialize)?;
    s.parse::<BigInt>().map_err(TezosError::DeserializeBigInt)
}

fn build_contract_operation(
    branch: &str,
    source: &str,
    counter: &BigInt,
    gas_limit: &BigInt,
    storage_limit: &BigInt,
    destination: &str,
    arguments: &Expr,
    signature: Option<&str>,
) -> Value {
    match signature {
        None => serde_json::json!(
            { "branch": branch
            , "contents":
                [
                    { "kind": "transaction"
                    , "source": source
                    , "fee": "0"
                    , "counter": counter.to_string()
                    , "gas_limit": gas_limit.to_string()
                    , "storage_limit": storage_limit.to_string()
                    , "amount": "0"
                    , "destination": destination
                    , "parameters":
                        { "entrypoint": "default"
                        , "value": arguments
                        }
                    }
                ]
            }
        ),
        Some(signature) => serde_json::json!(
            { "branch": branch
            , "contents":
                [
                    { "kind": "transaction"
                    , "source": source
                    , "fee": "0"
                    , "counter": counter.to_string()
                    , "gas_limit": gas_limit.to_string()
                    , "storage_limit": storage_limit.to_string()
                    , "amount": "0"
                    , "destination": destination
                    , "parameters":
                        { "entrypoint": "default"
                        , "value": arguments
                        }
                    }
                ]
            , "signature": signature
            }
        ),
    }
}

fn serialize_operation(host: &Url, op: Value) -> Result<String, TezosError> {
    let url = host
        .join("chains/main/blocks/head/helpers/forge/operations")
        .map_err(TezosError::UrlParse)?;

    println!("{}", op);

    ureq::post(url.as_str())
        .send_json(op)
        .into_json_deserialize()
        .map_err(TezosError::Deserialize)
}

fn dry_run_contract(host: &Url, op: Value, chain_id: &str) -> Result<Value, TezosError> {
    let url = host
        .join("chains/main/blocks/head/helpers/scripts/run_operation")
        .map_err(TezosError::UrlParse)?;

    let payload = serde_json::json!(
        { "operation": op
        , "chain_id": chain_id
        }
    );

    ureq::post(url.as_str())
        .send_json(payload)
        .into_json_deserialize()
        .map_err(TezosError::Deserialize)
}

fn main() -> Result<(), TezosError> {
    let node_host: Url =
        Url::parse("https://carthagenet.smartpy.io").map_err(TezosError::UrlParse)?;
    let source = "tz1cPQbVEBSygG5dwbqsaPCMpU4ZdyTzjy97";
    let destination = "KT1UnS3wvwcUnj3dFAikmM773byGjY5Ci2Lk";
    let secret_key = "edsk2yRWMofVt5oqk1BWP4tJGeWZ4ikoZJ4psdMzoBqyqpT9g8tvpk";

    let arguments = Expr::Prim {
        prim: "Right".into(),
        args: vec![Expr::Prim {
            prim: "Right".into(),
            args: vec![Expr::Prim {
                prim: "Pair".into(),
                args: vec![
                    Expr::Prim {
                        prim: "Some".into(),
                        args: vec![Expr::Bytes(vec![0xca, 0xfe, 0xba, 0xbe])],
                    },
                    Expr::Bytes(vec![0xca, 0xfe, 0xba, 0xbe]),
                ],
            }],
        }],
    };

    let s = serde_json::to_string(&arguments).unwrap();
    println!("{}", s);

    println!("{:?}", serde_json::from_str::<michelson::Expr>(&s));

    let counter = counter(&node_host, &source)?;

    let bootstrapped = bootstrapped(&node_host)?;

    println!("bootstrapped: {:?}", bootstrapped);

    let constants = constants(&node_host)?;

    println!("constants: {:?}", constants);

    let branch = head_hash(&node_host)?;

    println!("head hash: {}", branch);

    let chain_id = chain_id(&node_host)?;

    println!("chain_id: {}", chain_id);

    let op = build_contract_operation(
        &branch,
        &source,
        &counter,
        &constants.hard_gas_limit_per_operation,
        &constants.hard_storage_limit_per_operation,
        &destination,
        &arguments,
        None,
    );

    let sop = serialize_operation(&node_host, op)?;

    println!("serialized_operation: {}", &sop);

    let signature =
        crypto::sign_serialized_operation(&sop, secret_key).map_err(TezosError::Crypto)?;

    println!("signature: {}", signature);

    let signed_op = build_contract_operation(
        &branch,
        &source,
        &counter,
        &constants.hard_gas_limit_per_operation,
        &constants.hard_storage_limit_per_operation,
        &destination,
        &arguments,
        Some(&signature),
    );

    let dry_run_result = dry_run_contract(&node_host, signed_op, &chain_id)?;

    println!("dry_run_result: {:?}", dry_run_result);

    Ok(())
}
