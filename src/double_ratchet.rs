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
        csprng: &mut R,
        secret_key: X3DHSecretKey,
        recipient_prekey: &PrekeyPublicKey,
    ) -> DoubleRatchetClient {
        let receiving_ratchet_key = recipient_prekey.convert_to_ratchet_public_key();
        let sending_ratchet_keypair = RatchetKeyPair::new(csprng);

        // Here, we view the secret key derived from the X3DH key agreement
        // protocol as the intial root key.
        let mut root_key = RootKey(secret_key.0);
        let shared_secret = sending_ratchet_keypair.dh(&receiving_ratchet_key);

        // Here, we simultaneously derive both the sending chain key and the
        // new root key.
        let sending_chain_key = root_key.kdf(shared_secret);

        DoubleRatchetClient {
            sending_ratchet_keypair,
            receiving_ratchet_key: Some(receiving_ratchet_key),
            root_key,
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
            sending_ratchet_keypair,
            receiving_ratchet_key: None,
            root_key,
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
        // CR pandaman: document panic-freeness
        [
            x3dh_ad.0.clone(),
            bincode::serialize(&message_header).unwrap(),
        ]
        .concat()
    }

    fn encrypt(
        message_key: MessageKey,
        ciphertext: &[u8],
        associated_data: &[u8],
    ) -> Option<Vec<u8>> {
        let payload = Payload {
            msg: &ciphertext,
            aad: &associated_data,
        };

        let key = GenericArray::from_slice(&message_key.0);
        let nonce = GenericArray::from_slice(&message_key.1[0..12]);

        let cipher = Aes256Gcm::new(*key);
        cipher.encrypt(&nonce, payload).ok()
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
        // CR pandaman: document panic-freeness
        let ciphertext =
            DoubleRatchetClient::encrypt(message_key, plaintext, &associated_data).unwrap();

        self.sent_count += 1;

        // CR pandaman: document panic-freeness
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
        let nonce = GenericArray::from_slice(&message_key.1[0..12]);

        let cipher = Aes256Gcm::new(*key);
        cipher.decrypt(&nonce, payload).ok()
    }

    pub fn attempt_message_decryption<R: CryptoRng + RngCore>(
        &mut self,
        csprng: &mut R,
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
            new_state.sending_ratchet_keypair = RatchetKeyPair::new(csprng);
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
    use quickcheck::{Arbitrary, Gen};
    use rand::prelude::SliceRandom;
    use rand::rngs::OsRng;

    fn stub_x3dh() -> (X3DHClient, X3DHClient, X3DHSecretKey, X3DHAD) {
        let mut csprng = OsRng;
        let alice = X3DHClient::new(&mut csprng);
        let bob = X3DHClient::new(&mut csprng);

        let sender_info = b"alice";
        let receiver_info = b"bob";
        let associated_data = X3DHClient::build_associated_data(
            &alice.identity_key.public_key,
            &bob.identity_key.public_key,
            sender_info,
            receiver_info,
        );

        // We assume that Alice and Bob have already agreed upon some secret
        // key here.
        let mut secret_key = [0u8; 32];
        csprng.fill_bytes(&mut secret_key);
        (alice, bob, X3DHSecretKey(secret_key), associated_data)
    }

    fn copy_x3dh_secret_key(secret_key: &X3DHSecretKey) -> X3DHSecretKey {
        // We implement a weird cloning function here instead of deriving
        // Clone on X3DHSecretKey, as normal usage should never require cloning.
        X3DHSecretKey(secret_key.0.clone())
    }

    #[quickcheck]
    fn double_ratchet_one_message_works(message_content: Vec<u8>) -> bool {
        let mut csprng = OsRng;
        let (_alice_x3dh, bob_x3dh, secret_key, associated_data) = stub_x3dh();

        let mut alice = DoubleRatchetClient::initiate(
            &mut csprng,
            copy_x3dh_secret_key(&secret_key),
            &bob_x3dh.prekey.public_key,
        );
        let message = alice.encrypt_message(&message_content, &associated_data);

        let mut bob = DoubleRatchetClient::respond(secret_key, &bob_x3dh.prekey);
        let decrypted_message =
            bob.attempt_message_decryption(&mut csprng, &message, &associated_data);

        decrypted_message == Some(message_content)
    }

    #[derive(Debug, Clone)]
    enum Sender {
        Alice(bool),
        Bob(bool),
    }

    impl Sender {
        fn is_delivered(&self) -> bool {
            match self {
                Sender::Alice(b) => *b,
                Sender::Bob(b) => *b,
            }
        }
    }

    impl Arbitrary for Sender {
        fn arbitrary<G: Gen>(mut g: &mut G) -> Self {
            [
                Sender::Alice(bool::arbitrary(g)),
                Sender::Bob(bool::arbitrary(g)),
            ]
            .choose(&mut g)
            .expect("choose value")
            .clone()
        }
    }

    fn exchange_multiple_double_ratchet_messages(
        message_content: &[u8],
        sender_order: &[Sender],
    ) -> Vec<Option<Vec<u8>>> {
        let mut csprng = OsRng;
        let (_alice_x3dh, bob_x3dh, secret_key, associated_data) = stub_x3dh();

        // We use an empty message here, since the first message is already
        // covered by the double_ratchet_one_message_works quickcheck test.
        let empty_message = Vec::new();

        let mut alice = DoubleRatchetClient::initiate(
            &mut csprng,
            copy_x3dh_secret_key(&secret_key),
            &bob_x3dh.prekey.public_key,
        );
        let message = alice.encrypt_message(&empty_message, &associated_data);

        let mut bob = DoubleRatchetClient::respond(secret_key, &bob_x3dh.prekey);
        let decrypted_message =
            bob.attempt_message_decryption(&mut csprng, &message, &associated_data);

        assert_eq!(decrypted_message, Some(empty_message));

        let mut decrytion_results = Vec::new();
        for sender in sender_order.iter() {
            match sender {
                Sender::Alice(delivered) => {
                    let message = alice.encrypt_message(&message_content, &associated_data);
                    if *delivered {
                        let decrypted_message =
                            bob.attempt_message_decryption(&mut csprng, &message, &associated_data);
                        decrytion_results.push(decrypted_message);
                    } else {
                        decrytion_results.push(None);
                    }
                }
                Sender::Bob(delivered) => {
                    let message = bob.encrypt_message(&message_content, &associated_data);
                    if *delivered {
                        let decrypted_message = alice.attempt_message_decryption(
                            &mut csprng,
                            &message,
                            &associated_data,
                        );
                        decrytion_results.push(decrypted_message);
                    } else {
                        decrytion_results.push(None);
                    }
                }
            }
        }

        decrytion_results
    }

    #[quickcheck]
    fn double_ratchet_multiple_messages_works(
        message_content: Vec<u8>,
        sender_order: Vec<Sender>,
    ) -> bool {
        let results = exchange_multiple_double_ratchet_messages(&message_content, &sender_order);
        assert_eq!(results.len(), sender_order.len());
        results
            .iter()
            .zip(sender_order)
            .all(|(decrypted_message, sender)| {
                if sender.is_delivered() {
                    decrypted_message.as_ref() == Some(&message_content)
                } else {
                    decrypted_message == &None
                }
            })
    }

    #[test]
    fn responder_drops_first_message() {
        let message_content = Vec::new();
        let decrypted_messages = exchange_multiple_double_ratchet_messages(
            &message_content,
            &[Sender::Bob(false), Sender::Bob(true)],
        );
        assert_eq!(decrypted_messages, [None, Some(message_content.clone())]);
    }
}
