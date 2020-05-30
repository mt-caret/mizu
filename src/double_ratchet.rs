use crate::keys::{
    ChainKey, MessageKey, PrekeyKeyPair, PrekeyPublicKey, RatchetKeyPair, RatchetPublicKey, RootKey,
};
use crate::x3dh::{X3DHSecretKey, X3DHAD};
use aes_gcm::aead::{generic_array::GenericArray, Aead, NewAead, Payload};
use aes_gcm::Aes256Gcm;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

static MAX_SKIP: u64 = 32;

#[derive(Clone)]
pub struct DoubleRatchetClient {
    sending_ratchet_keypair: RatchetKeyPair,
    receiving_ratchet_key: Option<RatchetPublicKey>,
    root_key: RootKey,
    sending_chain_key: Option<ChainKey>,
    receiving_chain_key: Option<ChainKey>,
    sent_count: u64,
    received_count: u64,
    previous_sending_chain_count: u64,
    skipped_messages: HashMap<SkippedMessagesKey, MessageKey>,
}

// Since RatchetPublicKey is actually x25519_dalek's PublicKey and does not
// have an Hash trait implementation, we implement it here. As Eq is not
// implemented as a constant-time comparison, we purposefully do it here
// instead of on RatchetPublicKey to prevent potential misuse (resulting in
// timing attacks).
#[derive(PartialEq, Eq, Clone)]
pub struct SkippedMessagesKey(RatchetPublicKey, u64);
impl Hash for SkippedMessagesKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.0).0.as_bytes().hash(state);
        self.1.hash(state);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DoubleRatchetMessageHeader {
    ratchet_public_key: RatchetPublicKey,
    previous_sending_chain_count: u64,
    sent_count: u64,
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
        recipient_prekey: &PrekeyPublicKey,
    ) -> DoubleRatchetClient {
        let receiving_ratchet_key = recipient_prekey.convert_to_ratchet_public_key();
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
            sent_count: 0,
            received_count: 0,
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
            sent_count: 0,
            received_count: 0,
            previous_sending_chain_count: 0,
            skipped_messages: HashMap::new(),
        }
    }

    fn build_associated_data(
        x3dh_ad: &X3DHAD,
        message_header: &DoubleRatchetMessageHeader,
    ) -> Vec<u8> {
        [
            x3dh_ad.0.clone(),
            bincode::serialize(&message_header).unwrap(),
        ]
        .concat()
    }

    pub fn encrypt_message(&mut self, plaintext: &[u8], associated_data: &X3DHAD) -> Vec<u8> {
        let message_key = self
            .sending_chain_key
            .as_mut()
            .expect("sending chain key has not been initialized yet")
            .kdf();

        let message_header = DoubleRatchetMessageHeader {
            ratchet_public_key: self.sending_ratchet_keypair.public_key.clone(),
            sent_count: self.sent_count,
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
        bincode::serialize(&DoubleRatchetMessage {
            header: message_header,
            ciphertext: ciphertext,
        })
        .unwrap()
    }

    fn skip_message_keys(&mut self, until: u64) -> Option<()> {
        if self.received_count + MAX_SKIP < until {
            return None;
        }

        if let Some(receiving_chain_key) = self.receiving_chain_key.as_mut() {
            while self.received_count < until {
                let message_key = receiving_chain_key.kdf();
                // unwrapping self.receiving_ratchet_key here is safe,
                // since receiving_chain_key.is_some() implies
                // self.receiving_ratchet_key.is_some()
                self.skipped_messages.insert(
                    SkippedMessagesKey(
                        self.receiving_ratchet_key.clone().unwrap(),
                        self.received_count,
                    ),
                    message_key,
                );
                self.received_count += 1;
            }
        }
        Some(())
    }

    fn decrypt(
        message_key: MessageKey,
        ciphertext: &[u8],
        associated_data: &[u8],
    ) -> Option<Vec<u8>> {
        let payload = Payload {
            msg: &ciphertext,
            aad: &associated_data,
        };

        let key = GenericArray::from_slice(&message_key.0);
        let nonce = GenericArray::from_slice(&message_key.1);

        let cipher = Aes256Gcm::new(*key);
        cipher.decrypt(&nonce, payload).ok()
    }

    pub fn attempt_message_decryption<R: CryptoRng + RngCore>(
        &mut self,
        mut csprng: &mut R,
        serialized_message: &[u8],
        associated_data: &X3DHAD,
    ) -> Option<Vec<u8>> {
        let message: DoubleRatchetMessage = bincode::deserialize(serialized_message).ok()?;
        let associated_data =
            DoubleRatchetClient::build_associated_data(&associated_data, &message.header);

        // If the message header indicates a skipped message, remove the
        // corresponding message key, decrypt with it, and return. Remove
        // messages from self.skipped_messages only if decryption succeeds.
        let hashmap_key = SkippedMessagesKey(
            message.header.ratchet_public_key.clone(),
            message.header.sent_count,
        );
        if let Some(message_key) = self.skipped_messages.get(&hashmap_key) {
            let plaintext = DoubleRatchetClient::decrypt(
                message_key.clone(),
                &message.ciphertext,
                &associated_data,
            )?;
            assert!(self.skipped_messages.remove(&hashmap_key).is_some());
            return Some(plaintext);
        }

        let mut new_state = self.clone();

        // If the message has a new RatchetPublicKey, perform the DH ratchet.
        if Some(&message.header.ratchet_public_key) != new_state.receiving_ratchet_key.as_ref() {
            new_state.skip_message_keys(message.header.previous_sending_chain_count)?;

            new_state.previous_sending_chain_count = new_state.sent_count;
            new_state.sent_count = 0;
            new_state.received_count = 0;
            new_state.receiving_ratchet_key = Some(message.header.ratchet_public_key.clone());
            new_state.receiving_chain_key = Some(
                new_state.root_key.kdf(
                    new_state
                        .sending_ratchet_keypair
                        .dh(&message.header.ratchet_public_key),
                ),
            );
            new_state.sending_ratchet_keypair = RatchetKeyPair::new(&mut csprng);
            new_state.sending_chain_key = Some(
                new_state.root_key.kdf(
                    new_state
                        .sending_ratchet_keypair
                        .dh(&message.header.ratchet_public_key),
                ),
            );
        }

        new_state.skip_message_keys(message.header.sent_count)?;
        let message_key = new_state.receiving_chain_key.as_mut().unwrap().kdf();
        let plaintext =
            DoubleRatchetClient::decrypt(message_key, &message.ciphertext, &associated_data)?;
        new_state.received_count += 1;

        // Persist changes to the state only if decryption is successful.
        *self = new_state;
        Some(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x3dh::X3DHClient;
    use rand::rngs::OsRng;

    #[quickcheck]
    fn double_ratchet_one_message_works(message_content: Vec<u8>) -> bool {
        let mut csprng = OsRng;
        let alice_x3dh = X3DHClient::new(&mut csprng);
        let bob_x3dh = X3DHClient::new(&mut csprng);

        let sender_info = b"alice";
        let receiver_info = b"bob";
        let associated_data = X3DHClient::build_associated_data(
            &alice_x3dh.identity_key.public_key,
            &bob_x3dh.identity_key.public_key,
            sender_info,
            receiver_info,
        );

        // We assume that Alice and Bob have already agreed upon some secret
        // key here.
        let mut secret_key = [0u8; 32];
        csprng.fill_bytes(&mut secret_key);

        let mut alice = DoubleRatchetClient::initiate(
            &mut csprng,
            X3DHSecretKey(secret_key.clone()),
            &bob_x3dh.prekey.public_key,
        );
        let message = alice.encrypt_message(&message_content, &associated_data);

        let mut bob = DoubleRatchetClient::respond(X3DHSecretKey(secret_key), &bob_x3dh.prekey);
        let decrypted_message =
            bob.attempt_message_decryption(&mut csprng, &message, &associated_data);

        decrypted_message == Some(message_content)
    }
}
