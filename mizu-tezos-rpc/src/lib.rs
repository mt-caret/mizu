pub mod crypto;
mod helper;
pub mod michelson;

use michelson::Expr;
use num_bigint::{BigInt, BigUint};
use num_traits::Zero;
use serde::Deserialize;
use serde_json::Value;
use std::io;
use thiserror::Error;
use url::Url;

static PROTOCOL_CARTHAGE: &str = "PsCARTHAGazKbHtnKfLzQg3kms52kSRpgnDY982a9oYsSXRLQEb";

#[derive(Error, Debug)]
pub enum TezosError {
    #[error("failed to parse url: {0}")]
    UrlParse(url::ParseError),
    #[error("error: {0}")]
    IO(io::Error),
    #[error("deserialization error: {0} ({1})")]
    SerdeDeserialize(serde_json::error::Error, Value),
    #[error("deserialization error: {0}")]
    DeserializeBigInt(num_bigint::ParseBigIntError),
    #[error("crypto error: {0}")]
    Crypto(crypto::Error),
    #[error("tezos node rpc error: {0}")]
    Rpc(Value),
}

type Result<T> = std::result::Result<T, TezosError>;

#[derive(Deserialize, Debug)]
struct Bootstrapped {
    block: String,
    timestamp: String,
}

fn bootstrapped(host: &Url) -> Result<Bootstrapped> {
    let url = host
        .join("monitor/bootstrapped")
        .map_err(TezosError::UrlParse)?;

    ureq::get(url.as_str())
        .call()
        .into_json()
        .map_err(TezosError::IO)
        .and_then(|x| from_value(&x))
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

fn constants(host: &Url) -> Result<Constants> {
    let url = host
        .join("chains/main/blocks/head/context/constants")
        .map_err(TezosError::UrlParse)?;

    ureq::get(url.as_str())
        .call()
        .into_json()
        .map_err(TezosError::IO)
        .and_then(|x| from_value(&x))
}

fn head_hash(host: &Url) -> Result<String> {
    let url = host
        .join("chains/main/blocks/head/hash")
        .map_err(TezosError::UrlParse)?;

    ureq::get(url.as_str())
        .call()
        .into_json()
        .map_err(TezosError::IO)
        .and_then(|x| from_value(&x))
}

fn chain_id(host: &Url) -> Result<String> {
    let url = host
        .join("chains/main/chain_id")
        .map_err(TezosError::UrlParse)?;

    ureq::get(url.as_str())
        .call()
        .into_json()
        .map_err(TezosError::IO)
        .and_then(|x| from_value(&x))
}

fn parse_bigint(s: String) -> Result<BigInt> {
    s.parse::<BigInt>().map_err(TezosError::DeserializeBigInt)
}

fn counter(host: &Url, address: &str) -> Result<BigInt> {
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
        .into_json()
        .map_err(TezosError::IO)
        .and_then(|x| from_value(&x))?;
    parse_bigint(s)
}

#[derive(Debug)]
struct Operation {
    protocol: Option<String>,
    signature: Option<String>,
    branch: String,
    source: String,
    destination: String,
    fee: BigInt,
    counter: BigInt,
    gas_limit: BigInt,
    storage_limit: BigInt,
    parameters: Expr,
}

fn build_json(op: &Operation) -> Value {
    let mut value = serde_json::json!(
        { "branch": op.branch
            , "contents":
                [
                    { "kind": "transaction"
                    , "source": op.source
                    , "fee": op.fee.to_string()
                    , "counter": op.counter.to_string()
                    , "gas_limit": op.gas_limit.to_string()
                    , "storage_limit": op.storage_limit.to_string()
                    , "amount": "0"
                    , "destination": op.destination
                    , "parameters":
                        { "entrypoint": "default"
                        , "value": op.parameters
                        }
                    }
                ]
        }
    );

    if let Some(protocol) = &op.protocol {
        value
            .as_object_mut()
            .expect("value is an object")
            .insert("protocol".into(), Value::String(protocol.into()));
    }

    if let Some(signature) = &op.signature {
        value
            .as_object_mut()
            .expect("value is an object")
            .insert("signature".into(), Value::String(signature.into()));
    }

    value
}

fn serialize_operation(host: &Url, op: &Operation) -> Result<String> {
    let url = host
        .join("chains/main/blocks/head/helpers/forge/operations")
        .map_err(TezosError::UrlParse)?;

    let payload = build_json(op);

    ureq::post(url.as_str())
        .send_json(payload)
        .into_json()
        .map_err(TezosError::IO)
        .and_then(|x| from_value(&x))
}

#[derive(Debug)]
struct DryRunResult {
    consumed_gas: BigInt,
    paid_storage_size_diff: BigInt,
}

fn from_value<T>(value: &Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::value::from_value(value.clone())
        .map_err(|e| TezosError::SerdeDeserialize(e, value.clone()))
}

fn deserialize_bigint_from_value(value: &Value) -> Result<BigInt> {
    from_value(value).and_then(parse_bigint)
}

fn dry_run_contract(host: &Url, op: &Operation, chain_id: &str) -> Result<DryRunResult> {
    let url = host
        .join("chains/main/blocks/head/helpers/scripts/run_operation")
        .map_err(TezosError::UrlParse)?;

    let payload = serde_json::json!(
        { "operation": build_json(op)
        , "chain_id": chain_id
        }
    );

    let result: Value = ureq::post(url.as_str())
        .send_json(payload)
        .into_json()
        .map_err(TezosError::IO)
        .and_then(|x| from_value(&x))?;

    let op_result = &result["contents"][0]["metadata"]["operation_result"];
    let consumed_gas = op_result
        .get("consumed_gas")
        .map(deserialize_bigint_from_value)
        .unwrap_or_else(|| Ok(Zero::zero()))?;
    let paid_storage_size_diff = op_result
        .get("paid_storage_size_diff")
        .map(deserialize_bigint_from_value)
        .unwrap_or_else(|| Ok(Zero::zero()))?;

    Ok(DryRunResult {
        consumed_gas,
        paid_storage_size_diff,
    })
}

fn preapply_operation(host: &Url, op: &Operation) -> Result<Value> {
    let url = host
        .join("chains/main/blocks/head/helpers/preapply/operations")
        .map_err(TezosError::UrlParse)?;

    let payload = serde_json::json!(vec![build_json(op)]);

    ureq::post(url.as_str())
        .send_json(payload)
        .into_json()
        .map_err(TezosError::IO)
        .and_then(|x| from_value(&x))
}

fn inject_operation(host: &Url, signed_sop: &str) -> Result<String> {
    let url = host
        .join("injection/operation?chain=main")
        .map_err(TezosError::UrlParse)?;

    let payload = serde_json::json!(signed_sop);

    ureq::post(url.as_str())
        .send_json(payload)
        .into_json()
        .map_err(TezosError::IO)
        .and_then(|x| from_value(&x))
}

pub fn get_from_big_map(host: &Url, contract_address: &str, key: &str) -> Result<Expr> {
    let url = host
        .join(
            &[
                "chains/main/blocks/head/context/contracts/",
                contract_address,
                "/big_map_get",
            ]
            .concat(),
        )
        .map_err(TezosError::UrlParse)?;

    let payload = serde_json::json!(
    { "key": Expr::String(key.to_string())
    , "type": Expr::Prim {
            prim: "address".to_string(),
            args: Vec::new()
        }
    });

    ureq::post(url.as_str())
        .send_json(payload)
        .into_json()
        .map_err(TezosError::IO)
        .and_then(|x| from_value(&x))
}

// TODO: test remaining enums
#[derive(Debug)]
pub enum MizuOp {
    Post(Vec<Vec<u8>>, Vec<BigInt>),
    Poke(String, Vec<u8>),
    Register(Option<Vec<u8>>, Vec<u8>),
}

impl MizuOp {
    pub fn to_expr(&self) -> Expr {
        match self {
            MizuOp::Post(add, remove) => Expr::left(Expr::pair(
                Expr::List(add.iter().cloned().map(Expr::Bytes).collect()),
                Expr::List(remove.iter().cloned().map(Expr::nat).collect()),
            )),
            MizuOp::Poke(address, data) => Expr::right(Expr::left(Expr::pair(
                Expr::String(address.to_string()),
                Expr::Bytes(data.to_vec()),
            ))),
            MizuOp::Register(identity_key, prekey) => Expr::right(Expr::right(Expr::pair(
                Expr::some(identity_key.clone().map(Expr::Bytes)),
                Expr::Bytes(prekey.to_vec()),
            ))),
        }
    }
}

fn serialize_and_set_fee(host: &Url, op: &mut Operation, debug: bool) -> Result<String> {
    let sop = serialize_operation(&host, &op)?;

    if debug {
        eprintln!("serialized_operation: {}", &sop);
    }

    // sop is hex-encoded so we divide by 2 and add 64 bytes for the appended signature.
    let op_byte_length = sop.len() / 2 + 64;

    // currently hardcoded, since it seems we can't get these values programmatically:
    // https://gitlab.com/tezos/tezos/-/issues/425
    let minimal_fees = 100;
    let minimal_nanotez_per_gas_unit = 100;
    let minimal_nanotez_per_byte = 1000;

    let total_fee = (minimal_fees * 1000
        + minimal_nanotez_per_byte * op_byte_length
        + minimal_nanotez_per_gas_unit * op.gas_limit.clone())
        / 1000;

    if op.fee <= total_fee {
        op.fee = total_fee + 1;
        if debug {
            eprintln!("fee set to {}", op.fee);
        }
        serialize_and_set_fee(host, op, debug)
    } else {
        Ok(sop)
    }
}

// Code here was written based on the following sources:
// - https://www.ocamlpro.com/2018/11/15/an-introduction-to-tezos-rpcs-a-basic-wallet/
// - https://medium.com/chain-accelerator/how-to-use-tezos-rpcs-16c362f45d64
pub fn run_mizu_operation(
    host: &Url,
    parameters: &MizuOp,
    source: &str,
    destination: &str,
    secret_key: &str,
    debug: bool,
) -> Result<String> {
    let parameters = parameters.to_expr();
    let s = serde_json::to_string(&parameters).expect("serde should deserialize any MizuOp");
    if debug {
        eprintln!("{}", s);
        eprintln!("{:?}", serde_json::from_str::<michelson::Expr>(&s));
    }

    let counter = counter(&host, &source)? + 1;

    if debug {
        eprintln!("counter: {}", counter);
    }

    let bootstrapped = bootstrapped(&host)?;

    if debug {
        eprintln!("bootstrapped: {:?}", bootstrapped);
    }

    let constants = constants(&host)?;

    if debug {
        eprintln!("constants: {:?}", constants);
    }

    let branch = head_hash(&host)?;

    if debug {
        eprintln!("head hash: {}", branch);
    }

    let chain_id = chain_id(&host)?;

    if debug {
        eprintln!("chain_id: {}", chain_id);
    }

    let mut op = Operation {
        branch,
        source: source.to_string(),
        counter,
        fee: Zero::zero(),
        gas_limit: constants.hard_gas_limit_per_operation,
        storage_limit: constants.hard_storage_limit_per_operation,
        destination: destination.to_string(),
        parameters,
        protocol: None,
        signature: None,
    };

    let (dummy_signature, _) =
        crypto::sign_serialized_operation(&serialize_operation(&host, &op)?, &secret_key)
            .map_err(TezosError::Crypto)?;

    op.signature = Some(dummy_signature);

    let dry_run_result = dry_run_contract(&host, &op, &chain_id)?;

    if debug {
        eprintln!("consumed_gas: {}", dry_run_result.consumed_gas);
        eprintln!(
            "paid_storage_size_diff: {}",
            dry_run_result.paid_storage_size_diff
        );
    }

    op.gas_limit = dry_run_result.consumed_gas + 100;
    op.storage_limit = dry_run_result.paid_storage_size_diff + 20;
    op.signature = None;

    let sop = serialize_and_set_fee(&host, &mut op, debug)?;

    let (signature, raw_signature) =
        crypto::sign_serialized_operation(&sop, &secret_key).map_err(TezosError::Crypto)?;

    if debug {
        eprintln!("signature: {}", signature);
        eprintln!("raw_signature length: {}", raw_signature.len()); // 64
    }

    op.protocol = Some(PROTOCOL_CARTHAGE.to_string());
    op.signature = Some(signature);

    let preapply_result = preapply_operation(&host, &op)?;

    if preapply_result[0].get("id").is_some() {
        // some error occurred
        eprintln!("preapply error: {}", preapply_result);

        return Err(TezosError::Rpc(preapply_result));
    }

    if debug {
        eprintln!("preapply_result: {}", preapply_result);
    }

    let signed_sop = [sop, hex::encode(raw_signature)].concat();

    if debug {
        eprintln!("signed_sop: {}", signed_sop);
    }

    let hash = inject_operation(&host, &signed_sop)?;

    if debug {
        eprintln!("operation hash: {}", hash);
    }

    Ok(hash)
}

//fn main() -> Result<()> {
//    let node_host: Url =
//        Url::parse("https://carthagenet.smartpy.io").map_err(TezosError::UrlParse)?;
//    let source = "tz1RNhvTfU11uBkJ7ZLxRDn25asLj4tj7JJB";
//    let destination = "KT1UnS3wvwcUnj3dFAikmM773byGjY5Ci2Lk";
//    let secret_key = "edsk2yRWMofVt5oqk1BWP4tJGeWZ4ikoZJ4psdMzoBqyqpT9g8tvpk";
//
//    let parameters = MizuOp::Register(
//        Some(vec![
//            0xca, 0xfe, 0xba, 0xbe, 0xca, 0xfe, 0xba, 0xbe, 0xca, 0xfe, 0xba, 0xbe,
//        ]),
//        vec![
//            0xca, 0xfe, 0xba, 0xbe, 0xca, 0xfe, 0xba, 0xbe, 0xca, 0xfe, 0xba, 0xbe,
//        ],
//    );
//
//    let hash = run_mizu_operation(
//        &node_host,
//        &parameters,
//        source,
//        destination,
//        secret_key,
//        true,
//    )?;
//
//    println!("hash: {}", hash);
//
//    let user_data = get_from_big_map(&node_host, destination, source)?;
//
//    println!("user_data: {:?}", user_data);
//
//    Ok(())
//}
