use crate::keys::{ChainKey, MessageKey, PrekeyKeyPair, RatchetKeyPair, RatchetPublicKey, RootKey};
use crate::x3dh::{X3DHSecretKey, X3DHAD};
use aes_gcm::aead::{generic_array::GenericArray, Aead, NewAead, Payload};
use aes_gcm::Aes256Gcm;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct DoubleRatchetClient {
    sending_ratchet_keypair: RatchetKeyPair,
    receiving_ratchet_key: Option<RatchetPublicKey>,
    root_key: RootKey,
    sending_chain_key: Option<ChainKey>,
    receiving_chain_key: Option<ChainKey>,
    sent_message_count: u64,
    received_message_count: u64,
    previous_sending_chain_count: u64,
    skipped_messages: HashMap<(RatchetPublicKey, u64), MessageKey>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DoubleRatchetMessageHeader {
    ratchet_public_key: RatchetPublicKey,
    previous_sending_chain_count: u64,
    sent_message_count: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DoubleRatchetMessage {
    header: DoubleRatchetMessageHeader,
    ciphertext: Vec<u8>,
}

impl DoubleRatchetClient {
    pub fn initiate<R: CryptoRng + RngCore>(
        mut csprng: &mut R,
        secret_key: X3DHSecretKey,
        receiving_ratchet_key: RatchetPublicKey,
    ) -> DoubleRatchetClient {
        let sending_ratchet_keypair = RatchetKeyPair::new(&mut csprng);

        // Here, we view the secret key derived from the X3DH key agreement
        // protocol as the intial root key.
        let mut root_key = RootKey(secret_key.0);
        let shared_secret = sending_ratchet_keypair.dh(&receiving_ratchet_key);

        // Here, we simultaneously derive both the sending chain key and the
        // new root key.
        let sending_chain_key = root_key.kdf(shared_secret);

        DoubleRatchetClient {
            sending_ratchet_keypair: sending_ratchet_keypair,
            receiving_ratchet_key: Some(receiving_ratchet_key),
            root_key: root_key,
            sending_chain_key: Some(sending_chain_key),
            receiving_chain_key: None,
            sent_message_count: 0,
            received_message_count: 0,
            previous_sending_chain_count: 0,
            skipped_messages: HashMap::new(),
        }
    }

    pub fn respond(
        secret_key: X3DHSecretKey,
        prekey_keypair: &PrekeyKeyPair,
    ) -> DoubleRatchetClient {
        let sending_ratchet_keypair = prekey_keypair.convert_to_ratchet_keypair();
        let root_key = RootKey(secret_key.0);

        DoubleRatchetClient {
            sending_ratchet_keypair: sending_ratchet_keypair,
            receiving_ratchet_key: None,
            root_key: root_key,
            sending_chain_key: None,
            receiving_chain_key: None,
            sent_message_count: 0,
            received_message_count: 0,
            previous_sending_chain_count: 0,
            skipped_messages: HashMap::new(),
        }
    }

    fn build_associated_data(
        x3dh_ad: X3DHAD,
        message_header: &DoubleRatchetMessageHeader,
    ) -> Vec<u8> {
        [x3dh_ad.0, bincode::serialize(&message_header).unwrap()].concat()
    }

    pub fn encrypt_message(
        &mut self,
        plaintext: &[u8],
        associated_data: X3DHAD,
    ) -> DoubleRatchetMessage {
        let message_key = self
            .sending_chain_key
            .as_mut()
            .expect("sending chain key has not been initialized yet")
            .kdf();

        let message_header = DoubleRatchetMessageHeader {
            ratchet_public_key: self.sending_ratchet_keypair.public_key.clone(),
            sent_message_count: self.sent_message_count,
            previous_sending_chain_count: self.previous_sending_chain_count,
        };

        let associated_data =
            DoubleRatchetClient::build_associated_data(associated_data, &message_header);
        let payload = Payload {
            msg: plaintext,
            aad: &associated_data,
        };

        let key = GenericArray::from_slice(&message_key.0);
        let nonce = GenericArray::from_slice(&message_key.1);

        let cipher = Aes256Gcm::new(*key);
        let ciphertext = cipher.encrypt(&nonce, payload).unwrap();
        DoubleRatchetMessage {
            header: message_header,
            ciphertext: ciphertext,
        }
    }

    pub fn attempt_message_decryption(&mut self) {
        unimplemented!()
    }
}
