use base58check::{FromBase58Check, ToBase58Check};
use blake2::Blake2b;
use ed25519_dalek::{Keypair, PublicKey, SecretKey};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to decode hexstring: {0}")]
    HexDecode(hex::FromHexError),
    #[error("invalid key type")]
    KeyType,
    #[error("invalid key content (base58check): {0:?}")]
    KeyContent(base58check::FromBase58CheckError),
    #[error("ed25519 error: {0:?}")]
    Ed25519(ed25519_dalek::SignatureError),
}

fn base58check_decode(input: &str) -> Result<Vec<u8>, Error> {
    let (head, rest) = input.from_base58check().map_err(Error::KeyContent)?;
    Ok([vec![head], rest].concat())
}

pub fn sign_serialized_operation(
    serialized_operation: &str,
    public_key: &str,
    secret_key: &str,
) -> Result<String, Error> {
    let op = hex::decode(&serialized_operation).map_err(Error::HexDecode)?;

    if &secret_key[0..4] != "edsk" || &public_key[0..4] != "edpk" {
        return Err(Error::KeyType);
    }

    let secret_key = base58check_decode(secret_key)?;
    let secret = SecretKey::from_bytes(&secret_key[4..36]).map_err(Error::Ed25519)?;

    let public_key = base58check_decode(public_key)?;
    let public = PublicKey::from_bytes(&public_key[4..36]).map_err(Error::Ed25519)?;

    // TODO: weird error here if [u8; 64] is used instead of a Vec<u8>
    let signature: Vec<u8> = (Keypair { secret, public })
        .sign::<Blake2b>(&[vec![0x03], op].concat())
        .to_bytes()
        .iter()
        .cloned()
        .collect();
    Ok(signature[1..].to_base58check(signature[0]))
}
