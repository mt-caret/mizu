mod helper;
mod michelson;

use michelson::{Expr, Prim};
use num_bigint::{BigInt, BigUint};
use serde::{Deserialize, Serialize};
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

fn counter(host: &Url, contract_id: &str) -> Result<BigInt, TezosError> {
    let url = host
        .join(
            &[
                "chains/main/blocks/head/context/contracts/",
                contract_id,
                "/counter",
            ]
            .concat(),
        )
        .map_err(TezosError::UrlParse)?;

    ureq::get(url.as_str())
        .call()
        .into_json_deserialize()
        .map_err(TezosError::Deserialize)
}

//fn build_contract_operation(
//    branch: &str,
//    source: &str,
//    counter: &BigInt,
//    gas_limit: &BigInt,
//    storage_limit: &BigInt,
//    destination: &str,
//    arguments: &MichelsonExpr,
//) -> Value {
//    serde_json::json!(
//        { "branch": branch
//        , "contents":
//            [
//                { "kind": "transaction"
//                , "source": source
//                , "counter": counter
//                , "gas_limit": gas_limit
//                , "storage_limit": storage_limit
//                , "amount": 0
//                , "destination": destination
//                , "parameters":
//                    { "entrypoint": "default"
//                    , "value": arguments
//                    }
//                }
//            ]
//        }
//    )
//}
//
//fn serialize_operation(host: &Url, op: Value) -> Result<String, TezosError> {
//    let url = host
//        .join("chains/main/chain_id")
//        .map_err(TezosError::UrlParse)?;
//
//    ureq::post(url.as_str())
//        .send_json(op)
//        .into_json_deserialize()
//        .map_err(TezosError::Deserialize)
//}
//
//fn dry_run_contract(
//    host: &Url,
//    chain_id: &str,
//    branch: &str,
//    signature: &str,
//    source: &str,
//    counter: &BigInt,
//    gas_limit: &BigInt,
//    storage_limit: &BigInt,
//    destination: &str,
//    arguments: &MichelsonExpr,
//) -> Result<Value, TezosError> {
//    let url = host
//        .join("chains/main/chain_id")
//        .map_err(TezosError::UrlParse)?;
//
//    let payload = serde_json::json!(
//        { "operation":
//            { "branch": branch
//            , "contents":
//                [
//                    { "kind": "transaction"
//                    , "source": source
//                    , "counter": counter
//                    , "gas_limit": gas_limit
//                    , "storage_limit": storage_limit
//                    , "amount": 0
//                    , "destination": destination
//                    , "parameters":
//                      { "entrypoint": "default"
//                      , "value": arguments
//                    }
//                    }
//                ]
//            , "signature": signature
//            }
//        , "chain_id": chain_id
//        }
//    );
//
//    ureq::post(url.as_str())
//        .send_json(payload)
//        .into_json_deserialize()
//        .map_err(TezosError::Deserialize)
//}

fn main() -> Result<(), TezosError> {
    //let node_host: Url =
    //    Url::parse("https://carthagenet.smartpy.io").map_err(TezosError::UrlParse)?;
    //let source = ""; // TODO: fill
    //let contract_id = "tz1cPQbVEBSygG5dwbqsaPCMpU4ZdyTzjy97";
    //let destination = "KT1UnS3wvwcUnj3dFAikmM773byGjY5Ci2Lk";
    //    let counter = counter(&node_host, &contract_id)?;

    //let bootstrapped = bootstrapped(&node_host)?;

    //println!("bootstrapped: {:?}", bootstrapped);

    //let constants = constants(&node_host)?;

    //println!("constants: {:?}", constants);

    //let branch = head_hash(&node_host)?;

    //println!("head hash: {}", branch);

    //let chain_id = chain_id(&node_host)?;

    //println!("chain_id: {}", chain_id);

    //let op = build_contract_operation(branch, &source, &counter, &constants.hard_gas_limit_per_operation, &constants.hard_storage_limit_per_operation, &destination

    //println!(
    //    "dry_run_result: {}",
    //    dry_run_contract(&node_host, &chain_id, &branch)?
    //);

    let arguments = Prim::new(
        "Right",
        &[Expr::prim(
            "Right",
            &[Expr::prim(
                "Pair",
                &[
                    Expr::prim("Some", &[Expr::Bytes(vec![0xca, 0xfe, 0xba, 0xbe])]),
                    Expr::Bytes(vec![0xca, 0xfe, 0xba, 0xbe]),
                ],
            )],
        )],
    );

    let s = serde_json::to_string(&arguments).unwrap();
    println!("{}", s);

    println!("{:?}", serde_json::from_str::<michelson::Expr>(&s));

    Ok(())
}
