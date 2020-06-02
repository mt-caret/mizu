use crate::error::CryptoError;
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
    // TODO: Note that for Mizu, it's hard to imagine circumstances where
    // there are a large number of skipped messages, since the only conceivable
    // out-of-order / lost message scenario is when both communicating parties
    // send messages at or close to the same time. If we persist skipped
    // messages for indefinite amounts of time, this may result in a
    // "space leak". I'm curious as to what Signal does in this case.
    // Possibly some sort of higher-level protocol to forget or request
    // lost messages?
    skipped_messages: HashMap<SkippedMessagesKey, MessageKey>,
}

// Since RatchetPublicKey is actually x25519_dalek's PublicKey and does not
// have an Hash trait implementation, we implement it here. As Eq is not
// implemented as a constant-time comparison, we purposefully do it here
// instead of on RatchetPublicKey to prevent potential misuse (resulting in
// timing attacks).
#[derive(PartialEq, Eq, Clone)]
pub struct SkippedMessagesKey(RatchetPublicKey, u64);
// Clippy is concerned about implementing Hash but deriving PartialEq as
// k1 == k2 ⇒ hash(k1) == hash(k2) may not hold. However, since the
// implementation of hash is simple enough that it's relatively easy to see
// that the above property should always hold.
//
// TODO: implementing PartialEq over cryptographic primitives as constant-time
// compares may obsolete this issue altogether.
#[allow(clippy::derive_hash_xor_eq)]
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
        secret_key: &X3DHSecretKey,
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
        [
            x3dh_ad.0.clone(),
            // The only values that are serialized here (i.e. the fields of
            // DoubleRatchetMessageHeader) are u64s and a RatchetPublicKey
            // which is just an array of bytes, so it's probably safe to
            // unwrap() this.
            bincode::serialize(&message_header).unwrap(),
        ]
        .concat()
    }

    fn encrypt(
        message_key: MessageKey,
        ciphertext: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, aes_gcm::aead::Error> {
        let payload = Payload {
            msg: &ciphertext,
            aad: &associated_data,
        };

        let key = GenericArray::from_slice(&message_key.0);
        let nonce = GenericArray::from_slice(&message_key.1[0..12]);

        let cipher = Aes256Gcm::new(*key);
        cipher.encrypt(&nonce, payload)
    }

    pub fn encrypt_message(
        &mut self,
        plaintext: &[u8],
        associated_data: &X3DHAD,
    ) -> Result<DoubleRatchetMessage, CryptoError> {
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
        let ciphertext = DoubleRatchetClient::encrypt(message_key, plaintext, &associated_data)
            .map_err(|_| CryptoError::AEADEncryption("DoubleRatchetMessage".to_string()))?;

        self.sent_count += 1;

        Ok(DoubleRatchetMessage {
            header: message_header,
            ciphertext,
        })
    }

    pub fn encrypt_message_and_serialize(
        &mut self,
        plaintext: &[u8],
        associated_data: &X3DHAD,
    ) -> Result<Vec<u8>, CryptoError> {
        let message = self.encrypt_message(plaintext, associated_data)?;
        bincode::serialize(&message)
            .map_err(|err| CryptoError::Serialization("DoubleRatchetMessage".to_string(), *err))
    }

    fn skip_message_keys(&mut self, until: u64) -> Result<(), CryptoError> {
        if self.received_count + MAX_SKIP < until {
            return Err(CryptoError::TooManySkippedMessages);
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
        Ok(())
    }

    fn decrypt(
        message_key: MessageKey,
        ciphertext: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, aes_gcm::aead::Error> {
        let payload = Payload {
            msg: &ciphertext,
            aad: &associated_data,
        };

        let key = GenericArray::from_slice(&message_key.0);
        let nonce = GenericArray::from_slice(&message_key.1[0..12]);

        let cipher = Aes256Gcm::new(*key);
        cipher.decrypt(&nonce, payload)
    }

    pub fn attempt_message_decryption<R: CryptoRng + RngCore>(
        &mut self,
        csprng: &mut R,
        message: &DoubleRatchetMessage,
        associated_data: &X3DHAD,
    ) -> Result<Vec<u8>, CryptoError> {
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
            )
            .map_err(|_| CryptoError::AEADDecryption("DoubleRatchetMessage".to_string()))?;
            assert!(self.skipped_messages.remove(&hashmap_key).is_some());
            return Ok(plaintext);
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
            DoubleRatchetClient::decrypt(message_key, &message.ciphertext, &associated_data)
                .map_err(|_| CryptoError::AEADDecryption("DoubleRatchetMessage".to_string()))?;
        new_state.received_count += 1;

        // Persist changes to the state only if decryption is successful.
        *self = new_state;
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Sender;
    use crate::x3dh::X3DHClient;
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
            &copy_x3dh_secret_key(&secret_key),
            &bob_x3dh.prekey.public_key,
        );
        let message = alice
            .encrypt_message(&message_content, &associated_data)
            .expect("encryption should succeed");

        let mut bob = DoubleRatchetClient::respond(secret_key, &bob_x3dh.prekey);
        let decrypted_message = bob
            .attempt_message_decryption(&mut csprng, &message, &associated_data)
            .expect("decryption should succeed");

        decrypted_message == message_content
    }

    fn exchange_multiple_double_ratchet_messages(
        message_content: &[u8],
        sender_order: &[(Sender, bool)],
    ) -> Vec<Option<Vec<u8>>> {
        let mut csprng = OsRng;
        let (_alice_x3dh, bob_x3dh, secret_key, associated_data) = stub_x3dh();

        // We use an empty message here, since the first message is already
        // covered by the double_ratchet_one_message_works quickcheck test.
        let empty_message = Vec::new();

        let mut alice = DoubleRatchetClient::initiate(
            &mut csprng,
            &copy_x3dh_secret_key(&secret_key),
            &bob_x3dh.prekey.public_key,
        );
        let message = alice
            .encrypt_message(&empty_message, &associated_data)
            .expect("encryption should succeed");

        let mut bob = DoubleRatchetClient::respond(secret_key, &bob_x3dh.prekey);
        let decrypted_message = bob
            .attempt_message_decryption(&mut csprng, &message, &associated_data)
            .expect("decryption should succeed");

        assert_eq!(decrypted_message, empty_message);

        // TODO: it might be better here to add some numbering information to
        // the messages to make sure decryption of old messages isn't happening.
        let mut decrytion_results = Vec::new();
        for (sender, delivered) in sender_order.iter() {
            match sender {
                Sender::Alice => {
                    let message = alice
                        .encrypt_message(&message_content, &associated_data)
                        .expect("encryption should succeed");
                    if *delivered {
                        let decrypted_message =
                            bob.attempt_message_decryption(&mut csprng, &message, &associated_data);
                        decrytion_results.push(decrypted_message.ok());
                    } else {
                        decrytion_results.push(None);
                    }
                }
                Sender::Bob => {
                    let message = bob
                        .encrypt_message(&message_content, &associated_data)
                        .expect("encryption should succeed");
                    if *delivered {
                        let decrypted_message = alice.attempt_message_decryption(
                            &mut csprng,
                            &message,
                            &associated_data,
                        );
                        decrytion_results.push(decrypted_message.ok());
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
        sender_order: Vec<(Sender, bool)>,
    ) -> bool {
        let results = exchange_multiple_double_ratchet_messages(&message_content, &sender_order);
        assert_eq!(results.len(), sender_order.len());
        results
            .iter()
            .zip(sender_order)
            .all(|(decrypted_message, (_, delivered))| {
                if delivered {
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
            &[(Sender::Bob, false), (Sender::Bob, true)],
        );
        assert_eq!(decrypted_messages, [None, Some(message_content.clone())]);
    }
}
