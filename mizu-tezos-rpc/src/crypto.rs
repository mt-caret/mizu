use base58check::{FromBase58Check, ToBase58Check};
use blake2::VarBlake2b;
use digest::{Update, VariableOutput};
use serde::{Deserialize, Serialize};
use signatory_ring::ed25519;
use signature::Signer;
use std::fs::read_to_string;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to decode hexstring: {0}")]
    HexDecode(hex::FromHexError),
    #[error("invalid key type: expected {0} but found {1}")]
    KeyType(String, String),
    #[error("invalid key content (base58check): {0:?}")]
    KeyContent(base58check::FromBase58CheckError),
    #[error("invalid secret key length: expected 32 bytes but found {0} bytes")]
    SeedLength(usize),
    #[error("some error occured when creating signature")]
    Signature,
    #[error(
        "faucet file is invalid: an error occured when extracting secret key from faucet: {0:?}"
    )]
    ExtractSecretKey(failure::Error),
    #[error("faucet file is invalid: expected address {0} but found {1}")]
    AddressMismatch(String, String),
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
        return Err(Error::KeyType(
            "key starting with edpk".to_string(),
            public_key.to_string(),
        ));
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
        return Err(Error::KeyType(
            "key starting with edsk".to_string(),
            secret_key.to_string(),
        ));
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
        // Surprisingly, reading the whole file is faster.
        // https://github.com/serde-rs/json/issues/160#issuecomment-253446892
        Ok(serde_json::from_str(&read_to_string(path)?)?)
    }

    pub fn derive_secret_key(&self) -> Result<String, Error> {
        use bip39::{Language, Mnemonic, Seed};
        //use sha2::{Digest, Sha512};
        use sodiumoxide::crypto::sign::ed25519;

        let mnemonic = Mnemonic::from_phrase(&self.mnemonic.join(" "), Language::English)
            .map_err(Error::ExtractSecretKey)?;
        let seed = Seed::new(
            &mnemonic,
            &[self.email.clone(), self.password.clone()].concat(),
        );
        let seed = &seed.as_bytes()[0..32];
        eprintln!("seed: {}", hex::encode(&seed));
        let seed = ed25519::Seed::from_slice(seed).ok_or_else(|| Error::SeedLength(seed.len()))?;
        eprintln!("seed: {}", hex::encode(&seed.as_ref()));
        let (public_key, secret_key) = ed25519::keypair_from_seed(&seed);
        eprintln!("secret_key: {}", hex::encode(&secret_key.as_ref()));

        let edpk_prefix: &[u8] = &[0x0d, 0x0f, 0x25, 0xd9];
        let encoded_public_key = base58check_encode(&[edpk_prefix, public_key.as_ref()].concat());
        let address = derive_address_from_pubkey(&encoded_public_key)?;
        if address != self.pkh {
            return Err(Error::AddressMismatch(address, self.pkh.clone()));
        }

        //let mut hasher = Sha512::new();
        //Digest::update(&mut hasher, seed);
        //let hash = &hasher.finalize();
        //let mut lefthalf = hash[32..].to_vec();
        //lefthalf[0] &= 248;
        //lefthalf[31] &= 127;
        //lefthalf[31] |= 64;

        //let edsk_prefix = vec![0x2b, 0xf6, 0x4e, 0x07];
        //Ok(base58check_encode(&[edsk_prefix, lefthalf].concat()))

        let edsk_prefix = vec![0x2b, 0xf6, 0x4e, 0x07];
        Ok(base58check_encode(
            &[&edsk_prefix, &seed.as_ref()[0..32]].concat(),
        ))
    }
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

    #[test]
    fn derive_secret_key_works() {
        let faucet_output = r#"{
            "mnemonic": [
                "fence",
                "taxi",
                "verify",
                "guilt",
                "industry",
                "oval",
                "begin",
                "rain",
                "glide",
                "topic",
                "sting",
                "lava",
                "inside",
                "chief",
                "heavy"
            ],
            "secret": "9f424988a4b706c9d88a65a590ec6b2edd00e7c2",
            "amount": "12046537458",
            "pkh": "tz1RNhvTfU11uBkJ7ZLxRDn25asLj4tj7JJB",
            "password": "jqeUK02nwX",
            "email": "aotdzyuv.llfpztgq@tezos.example.org"
        }"#;
        let faucet_output: FaucetOutput = serde_json::from_str(faucet_output).unwrap();
        assert_eq!(
            faucet_output.derive_secret_key().unwrap(),
            "edsk2yRWMofVt5oqk1BWP4tJGeWZ4ikoZJ4psdMzoBqyqpT9g8tvpk"
        )
    }
}
