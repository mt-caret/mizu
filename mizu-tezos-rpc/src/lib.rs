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

use chrono::DateTime;
use mizu_tezos_interface::*;

static PROTOCOL_CARTHAGE: &str = "PsCARTHAGazKbHtnKfLzQg3kms52kSRpgnDY982a9oYsSXRLQEb";

#[derive(Error, Debug)]
pub enum RpcError {
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
    #[error("error when decoding user data: {0}")]
    UserData(String),
}

type Result<T> = std::result::Result<T, RpcError>;

#[derive(Deserialize, Debug)]
struct Bootstrapped {
    block: String,
    timestamp: String,
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

fn parse_bigint(s: String) -> Result<BigInt> {
    s.parse::<BigInt>().map_err(RpcError::DeserializeBigInt)
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
        .map_err(|e| RpcError::SerdeDeserialize(e, value.clone()))
}

fn deserialize_bigint_from_value(value: &Value) -> Result<BigInt> {
    from_value(value).and_then(parse_bigint)
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

#[derive(Debug)]
pub struct TezosRpc {
    debug: bool,
    host: Url,
    address: String,
    secret_key: String,
    contract_address: String,
}

impl TezosRpc {
    pub fn new(
        debug: bool,
        host: Url,
        address: String,
        secret_key: String,
        contract_address: String,
    ) -> Self {
        Self {
            debug,
            host,
            address,
            secret_key,
            contract_address,
        }
    }

    fn resolve_path(&self, path: &str) -> Result<Url> {
        self.host.join(path).map_err(RpcError::UrlParse)
    }

    fn bootstrapped(&self) -> Result<Bootstrapped> {
        let url = self.resolve_path("monitor/bootstrapped")?;

        ureq::get(url.as_str())
            .call()
            .into_json()
            .map_err(RpcError::IO)
            .and_then(|x| from_value(&x))
    }

    fn constants(&self) -> Result<Constants> {
        let url = self.resolve_path("chains/main/blocks/head/context/constants")?;

        ureq::get(url.as_str())
            .call()
            .into_json()
            .map_err(RpcError::IO)
            .and_then(|x| from_value(&x))
    }

    fn head_hash(&self) -> Result<String> {
        let url = self.resolve_path("chains/main/blocks/head/hash")?;

        ureq::get(url.as_str())
            .call()
            .into_json()
            .map_err(RpcError::IO)
            .and_then(|x| from_value(&x))
    }

    fn chain_id(&self) -> Result<String> {
        let url = self.resolve_path("chains/main/chain_id")?;

        ureq::get(url.as_str())
            .call()
            .into_json()
            .map_err(RpcError::IO)
            .and_then(|x| from_value(&x))
    }

    fn counter(&self) -> Result<BigInt> {
        let url = self.resolve_path(
            &[
                "chains/main/blocks/head/context/contracts/",
                &self.address,
                "/counter",
            ]
            .concat(),
        )?;

        let s: String = ureq::get(url.as_str())
            .call()
            .into_json()
            .map_err(RpcError::IO)
            .and_then(|x| from_value(&x))?;
        parse_bigint(s)
    }

    fn serialize_operation(&self, op: &Operation) -> Result<String> {
        let url = self.resolve_path("chains/main/blocks/head/helpers/forge/operations")?;

        let payload = build_json(op);

        ureq::post(url.as_str())
            .send_json(payload)
            .into_json()
            .map_err(RpcError::IO)
            .and_then(|x| from_value(&x))
    }

    fn dry_run_contract(&self, op: &Operation, chain_id: &str) -> Result<DryRunResult> {
        let url = self.resolve_path("chains/main/blocks/head/helpers/scripts/run_operation")?;

        let payload = serde_json::json!(
            { "operation": build_json(op)
            , "chain_id": chain_id
            }
        );

        let result: Value = ureq::post(url.as_str())
            .send_json(payload)
            .into_json()
            .map_err(RpcError::IO)
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

    fn preapply_operation(&self, op: &Operation) -> Result<Value> {
        let url = self.resolve_path("chains/main/blocks/head/helpers/preapply/operations")?;

        let payload = serde_json::json!(vec![build_json(op)]);

        ureq::post(url.as_str())
            .send_json(payload)
            .into_json()
            .map_err(RpcError::IO)
            .and_then(|x| from_value(&x))
    }

    fn inject_operation(&self, signed_sop: &str) -> Result<String> {
        let url = self.resolve_path("injection/operation?chain=main")?;

        let payload = serde_json::json!(signed_sop);

        ureq::post(url.as_str())
            .send_json(payload)
            .into_json()
            .map_err(RpcError::IO)
            .and_then(|x| from_value(&x))
    }

    pub fn get_from_big_map(&self, key: &str) -> Result<Option<Expr>> {
        let url = self.resolve_path(
            &[
                "chains/main/blocks/head/context/contracts/",
                &self.contract_address,
                "/big_map_get",
            ]
            .concat(),
        )?;

        let payload = serde_json::json!(
        { "key": Expr::String(key.to_string())
        , "type": Expr::Prim {
                prim: "address".to_string(),
                args: Vec::new()
            }
        });

        let value = ureq::post(url.as_str())
            .send_json(payload)
            .into_json()
            .map_err(RpcError::IO)?;

        if value.is_null() {
            Ok(None)
        } else {
            from_value(&value).map(Some)
        }
    }

    fn serialize_and_set_fee(&self, op: &mut Operation) -> Result<String> {
        let sop = self.serialize_operation(&op)?;

        if self.debug {
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
            if self.debug {
                eprintln!("fee set to {}", op.fee);
            }
            self.serialize_and_set_fee(op)
        } else {
            Ok(sop)
        }
    }

    // Code here was written based on the following sources:
    // - https://www.ocamlpro.com/2018/11/15/an-introduction-to-tezos-rpcs-a-basic-wallet/
    // - https://medium.com/chain-accelerator/how-to-use-tezos-rpcs-16c362f45d64
    pub fn run_mizu_operation(&self, parameters: &MizuOp) -> Result<String> {
        let parameters = parameters.to_expr();
        let s = serde_json::to_string(&parameters).expect("serde should deserialize any MizuOp");
        if self.debug {
            eprintln!("{}", s);
            eprintln!("{:?}", serde_json::from_str::<michelson::Expr>(&s));
        }

        let counter = self.counter()? + 1;

        if self.debug {
            eprintln!("counter: {}", counter);
        }

        let bootstrapped = self.bootstrapped()?;

        if self.debug {
            eprintln!("bootstrapped: {:?}", bootstrapped);
        }

        let constants = self.constants()?;

        if self.debug {
            eprintln!("constants: {:?}", constants);
        }

        let branch = self.head_hash()?;

        if self.debug {
            eprintln!("head hash: {}", branch);
        }

        let chain_id = self.chain_id()?;

        if self.debug {
            eprintln!("chain_id: {}", chain_id);
        }

        let mut op = Operation {
            branch,
            source: self.address.to_string(),
            counter,
            fee: Zero::zero(),
            gas_limit: constants.hard_gas_limit_per_operation,
            storage_limit: constants.hard_storage_limit_per_operation,
            destination: self.contract_address.to_string(),
            parameters,
            protocol: None,
            signature: None,
        };

        let (dummy_signature, _) =
            crypto::sign_serialized_operation(&self.serialize_operation(&op)?, &self.secret_key)
                .map_err(RpcError::Crypto)?;

        op.signature = Some(dummy_signature);

        let dry_run_result = self.dry_run_contract(&op, &chain_id)?;

        if self.debug {
            eprintln!("consumed_gas: {}", dry_run_result.consumed_gas);
            eprintln!(
                "paid_storage_size_diff: {}",
                dry_run_result.paid_storage_size_diff
            );
        }

        op.gas_limit = dry_run_result.consumed_gas + 100;
        op.storage_limit = dry_run_result.paid_storage_size_diff + 20;
        op.signature = None;

        let sop = self.serialize_and_set_fee(&mut op)?;

        let (signature, raw_signature) =
            crypto::sign_serialized_operation(&sop, &self.secret_key).map_err(RpcError::Crypto)?;

        if self.debug {
            eprintln!("signature: {}", signature);
            eprintln!("raw_signature length: {}", raw_signature.len()); // 64
        }

        op.protocol = Some(PROTOCOL_CARTHAGE.to_string());
        op.signature = Some(signature);

        let preapply_result = self.preapply_operation(&op)?;

        if preapply_result[0].get("id").is_some() {
            // some error occurred
            eprintln!("preapply error: {}", preapply_result);

            return Err(RpcError::Rpc(preapply_result));
        }

        if self.debug {
            eprintln!("preapply_result: {}", preapply_result);
        }

        let signed_sop = [sop, hex::encode(raw_signature)].concat();

        if self.debug {
            eprintln!("signed_sop: {}", signed_sop);
        }

        let hash = self.inject_operation(&signed_sop)?;

        if self.debug {
            eprintln!("operation hash: {}", hash);
        }

        Ok(hash)
    }
}

fn decode_bytes(value: &Value) -> Result<Vec<u8>> {
    let s = value
        .get("bytes")
        .ok_or_else(|| RpcError::UserData("expected bytes".to_string()))?
        .as_str()
        .ok_or_else(|| RpcError::UserData("expected string".to_string()))?;
    hex::decode(s).map_err(|e| RpcError::UserData(format!("{}", e)))
}

fn decode_message(value: &Value) -> Result<Message> {
    let content = decode_bytes(&value["args"][0])?;
    let timestamp_str = value["args"][1]
        .get("string")
        .ok_or_else(|| RpcError::UserData("expected string".to_string()))?
        .as_str()
        .ok_or_else(|| RpcError::UserData("expected string".to_string()))?;
    let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
        .map_err(|e| RpcError::UserData(format!("error parsing data: {}", e)))?
        .naive_utc();

    Ok(Message { content, timestamp })
}

fn parse_user_data(expr: &Expr) -> Result<UserData> {
    let value = serde_json::json!(expr);
    let identity_key = decode_bytes(&value["args"][0]["args"][0])?;
    let prekey = decode_bytes(&value["args"][0]["args"][1])?;
    let postal_box = value["args"][1]["args"][0]
        .as_array()
        .ok_or_else(|| RpcError::UserData("expected array".to_string()))?
        .iter()
        .map(decode_message)
        .collect::<Result<Vec<_>>>()?;
    let pokes = value["args"][1]["args"][1]
        .as_array()
        .ok_or_else(|| RpcError::UserData("expected array".to_string()))?
        .iter()
        .map(decode_bytes)
        .collect::<Result<Vec<_>>>()?;

    Ok(UserData {
        identity_key,
        prekey,
        postal_box,
        pokes,
    })
}

impl Tezos for TezosRpc {
    type ReadError = RpcError;
    type WriteError = RpcError;

    fn address(&self) -> &str {
        &self.address
    }

    fn secret_key(&self) -> &str {
        &self.secret_key
    }

    fn retrieve_user_data(
        &self,
        address: &str,
    ) -> std::result::Result<Option<UserData>, Self::ReadError> {
        let value = self.get_from_big_map(address)?;
        match value {
            None => Ok(None),
            Some(value) => parse_user_data(&value).map(Some),
        }
    }

    fn post(&self, add: &[&[u8]], remove: &[&usize]) -> std::result::Result<(), Self::WriteError> {
        let add = add.iter().map(|x| x.to_vec()).collect();
        let remove = remove.iter().map(|&&x| x.into()).collect();
        let op = MizuOp::Post(add, remove);

        let _hash = self.run_mizu_operation(&op)?;
        Ok(())
    }

    fn poke(&self, target_address: &str, data: &[u8]) -> std::result::Result<(), Self::WriteError> {
        let op = MizuOp::Poke(target_address.to_string(), data.to_vec());

        let _hash = self.run_mizu_operation(&op)?;
        Ok(())
    }

    fn register(
        &self,
        identity_key: Option<&[u8]>,
        prekey: &[u8],
    ) -> std::result::Result<(), Self::WriteError> {
        let op = MizuOp::Register(identity_key.map(|x| x.to_vec()), prekey.to_vec());

        let _hash = self.run_mizu_operation(&op)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_tezos_rpc() -> Result<TezosRpc> {
        Ok(TezosRpc {
            debug: false,
            host: Url::parse("https://carthagenet.smartpy.io").map_err(RpcError::UrlParse)?,
            address: "tz1RNhvTfU11uBkJ7ZLxRDn25asLj4tj7JJB".to_string(),
            secret_key: "edsk2yRWMofVt5oqk1BWP4tJGeWZ4ikoZJ4psdMzoBqyqpT9g8tvpk".to_string(),
            contract_address: "KT1UnS3wvwcUnj3dFAikmM773byGjY5Ci2Lk".to_string(),
        })
    }

    // This test writes data out to a contract every time it is run, so
    // shouldn't be called unnecessarily!
    #[test]
    #[ignore]
    fn contract_call_succeeds() -> Result<()> {
        let rpc = get_tezos_rpc()?;

        let parameters = MizuOp::Register(
            Some(vec![
                0xca, 0xfe, 0xba, 0xbe, 0xca, 0xfe, 0xba, 0xbe, 0xca, 0xfe, 0xba, 0xbe,
            ]),
            vec![
                0xca, 0xfe, 0xba, 0xbe, 0xca, 0xfe, 0xba, 0xbe, 0xca, 0xfe, 0xba, 0xbe,
            ],
        );

        assert!(rpc.run_mizu_operation(&parameters).is_ok());

        Ok(())
    }

    #[test]
    fn reads_work() -> Result<()> {
        let rpc = get_tezos_rpc()?;

        println!("{}", serde_json::json!(rpc.get_from_big_map(&rpc.address)?));
        assert!(rpc.get_from_big_map(&rpc.address).is_ok());

        Ok(())
    }

    #[test]
    fn reads_to_unknown_address() -> Result<()> {
        let rpc = get_tezos_rpc()?;

        assert!(rpc
            .get_from_big_map("tz1PtxhBALR5qE3heaR9AY8khUBCkuGwUKjA")
            .is_ok());

        Ok(())
    }
}
