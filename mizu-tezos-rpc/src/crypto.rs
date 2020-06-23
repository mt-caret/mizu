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

fn base58check_encode(input: &[u8]) -> String {
    input[1..].to_base58check(input[0])
}

// TODO: test this when turning this into a library later
pub fn derive_address_from_pubkey(public_key: &str) -> Result<String, Error> {
    if &public_key[0..4] != "edpk" {
        return Err(Error::KeyType);
    }
    let public_key = &base58check_decode(public_key)?[4..];

    let mut hasher = VarBlake2b::new(20).expect("20 byte output should be valid for blake2b");
    hasher.update(public_key);
    let hash = hasher.finalize_boxed();

    Ok(base58check_encode(
        &[vec![6, 161, 159], hash.to_vec()].concat(),
    ))
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
    let signer: ed25519::Signer = (&ed25519::Seed::from_bytes(&secret_key)
        .ok_or_else(|| Error::SeedLength(secret_key.len()))?)
        .into();

    let mut hasher = VarBlake2b::new(32).expect("32 byte output should be valid for blake2b");
    hasher.update(&[vec![0x03], op].concat());
    let hash = hasher.finalize_boxed();

    let signature = signer
        .try_sign(&hash)
        .map_err(|_| Error::Signature)?
        .to_bytes();

    Ok((
        base58check_encode(&[vec![0x09, 0xf5, 0xcd, 0x86, 0x12], signature.to_vec()].concat()),
        signature.to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_address_from_pubkey_works() -> Result<(), Error> {
        let address =
            derive_address_from_pubkey("edpkuwY2nMXEdzhKd9PxsBfX4ZxZ78w2yoTbEN6yfq5HCGx1MnxDdj")?;
        assert_eq!(address, "tz1RNhvTfU11uBkJ7ZLxRDn25asLj4tj7JJB");
        Ok(())
    }

    #[test]
    fn sign_serialized_operation_works() -> Result<(), Error> {
        let sop = "ce69c5713dac3537254e7be59759cf59c15abd530d10501ccf9028a5786314cf08000002298c03ed7d454a101eb7022bc95f7e5f41ac78d0860303c8010080c2d72f0000e7670f32038107a59a2b9cfefae36ea21f5aa63c00";
        let secret_key = "edsk3gUfUPyBSfrS9CCgmCiQsTCHGkviBDusMxDJstFtojtc1zcpsh";
        let (signature, raw_signature) = sign_serialized_operation(sop, secret_key)?;
        assert_eq!(signature,
            "edsigtkpiSSschcaCt9pUVrpNPf7TTcgvgDEDD6NCEHMy8NNQJCGnMfLZzYoQj74yLjo9wx6MPVV29CvVzgi7qEcEUok3k7AuMg");
        assert_eq!(hex::encode(raw_signature),
            "637e08251cae646a42e6eb8bea86ece5256cf777c52bc474b73ec476ee1d70e84c6ba21276d41bc212e4d878615f4a31323d39959e07539bc066b84174a8ff0d");
        Ok(())
    }
}
