use base58check::{FromBase58Check, ToBase58Check};
use blake2::VarBlake2b;
use digest::{Update, VariableOutput};
use signatory_ring::ed25519;
use signature::Signer;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to decode hexstring: {0}")]
    HexDecode(hex::FromHexError),
    #[error("invalid key type")]
    KeyType,
    #[error("invalid key content (base58check): {0:?}")]
    KeyContent(base58check::FromBase58CheckError),
    #[error("invalid secret key length: expected 32 bytes but found {0} bytes")]
    SeedLength(usize),
    #[error("some error occured when creating signature")]
    Signature,
}

fn base58check_decode(input: &str) -> Result<Vec<u8>, Error> {
    let (head, rest) = input.from_base58check().map_err(Error::KeyContent)?;
    Ok([vec![head], rest].concat())
}

// Based on https://www.ocamlpro.com/2018/11/21/an-introduction-to-tezos-rpcs-signing-operations/
pub fn sign_serialized_operation(
    serialized_operation: &str,
    secret_key: &str,
) -> Result<(String, Vec<u8>), Error> {
    let op = hex::decode(&serialized_operation).map_err(Error::HexDecode)?;

    if &secret_key[0..4] != "edsk" {
        return Err(Error::KeyType);
    }

    let secret_key = &base58check_decode(secret_key)?[4..];
    println!("secret_key: {}", hex::encode(&secret_key));
    let signer: ed25519::Signer = (&ed25519::Seed::from_bytes(&secret_key)
        .ok_or_else(|| Error::SeedLength(secret_key.len()))?)
        .into();

    let mut hasher = VarBlake2b::new(32).expect("32 byte output should be valid for blake2b");
    hasher.update(&[vec![0x03], op].concat());
    let hash = hasher.finalize_boxed();
    println!("hash: {}", hex::encode(&hash));

    let signature = signer
        .try_sign(&hash)
        .map_err(|_| Error::Signature)?
        .to_bytes();

    println!("sig: {}", hex::encode(&signature.to_vec()));

    Ok((
        [vec![0xf5, 0xcd, 0x86, 0x12], signature.to_vec()]
            .concat()
            .to_base58check(0x09),
        signature.to_vec(),
    ))
}
