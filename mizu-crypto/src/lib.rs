#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

pub mod double_ratchet;
pub mod error;
pub mod keys;
pub mod x3dh;

use double_ratchet::{DoubleRatchetClient, DoubleRatchetMessage};
use error::CryptoError;
use keys::{EphemeralPublicKey, IdentityPublicKey, PrekeyPublicKey};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use x3dh::{X3DHClient, X3DHMessage, X3DHSecretKey};

// TODO: We use serde and bincode to serialize messages.
// This creates a potential issue: is it possible to differentiate
// between types of messages (i.e. does bincode leave some sort of
// marker so the type of serialized data can be identified)?
// If it is, then anybody can see which types of messages are being
// sent, which when combined with message size, can be considered to
// be a case of nontrivial metadata leakage.
//
// How bincode works seems pretty straightforward:
// http://tyoverby.com/posts/bincode_release.html
//
// TODO: Are the IdentityPublicKeys in all messages really necessary?
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    X3DH(X3DHMessage),
    Regular(IdentityPublicKey, DoubleRatchetMessage),
}

// TODO: What happens when each side creates and sends a X3DH message for the other?
// TODO: there needs to be some way to persist this to disk
#[derive(Serialize, Deserialize)]
pub struct Client {
    x3dh: X3DHClient,
    double_ratchet: Option<DoubleRatchetClient>,
    our_info: Vec<u8>,
    their_info: Vec<u8>,
    unacknowledged_x3dh: Option<(X3DHSecretKey, EphemeralPublicKey)>,
}

impl Client {
    pub fn new<R: CryptoRng + RngCore>(
        csprng: &mut R,
        our_info: &[u8],
        their_info: &[u8],
    ) -> Client {
        Client {
            x3dh: X3DHClient::new(csprng),
            double_ratchet: None,
            our_info: our_info.to_vec(),
            their_info: their_info.to_vec(),
            unacknowledged_x3dh: None,
        }
    }

    pub fn with_x3dh_client(x3dh_client: X3DHClient, our_info: &[u8], their_info: &[u8]) -> Client {
        Client {
            x3dh: x3dh_client,
            double_ratchet: None,
            our_info: our_info.to_vec(),
            their_info: their_info.to_vec(),
            unacknowledged_x3dh: None,
        }
    }

    pub fn create_message<R: CryptoRng + RngCore>(
        &mut self,
        csprng: &mut R,
        recipient_identity_key: &IdentityPublicKey,
        recipient_prekey: &PrekeyPublicKey,
        message_content: &[u8],
    ) -> Result<Message, CryptoError> {
        let ad = X3DHClient::build_associated_data(
            &self.x3dh.identity_key.public_key,
            &recipient_identity_key,
            &self.our_info,
            &self.their_info,
        );
        match (
            self.double_ratchet.as_mut(),
            self.unacknowledged_x3dh.clone(),
        ) {
            // If we don't have a DoubleRatchetClient, then we initiate X3DH
            // and set up DoubleRatchetClient. In case that this message is
            // lost, we continue to wrap all subsequent DoubleRatchetMessages
            // with the same X3DHMessage until we receive a response, at which
            // point it's safe to just send DoubleRatchetMessages on their own.
            (None, None) => {
                let (secret_key, ephemeral_public_key) =
                    self.x3dh
                        .derive_initial_keys(csprng, recipient_identity_key, recipient_prekey);
                let mut double_ratchet =
                    DoubleRatchetClient::initiate(csprng, &secret_key, recipient_prekey);
                let serialized_message =
                    double_ratchet.encrypt_message_and_serialize(message_content, &ad)?;
                let x3dh_message = self.x3dh.construct_initial_message(
                    &serialized_message,
                    &secret_key,
                    &ephemeral_public_key,
                    ad,
                );

                self.double_ratchet = Some(double_ratchet);
                self.unacknowledged_x3dh = Some((secret_key, ephemeral_public_key));
                Ok(Message::X3DH(x3dh_message))
            }
            // Since we only set the X3DH keys when we set up
            // DoubleRatchetClient, this branch should never be taken.
            (None, Some(_)) => {
                unreachable!("Missing DoubleRatchetClient with unacknowledged X3DH message");
            }
            // This is the most uninteresting branch, where the X3DHMessage
            // has been acknowledged and we're just sending
            // DoubleRatchetMessages.
            (Some(double_ratchet), None) => {
                let double_ratchet_message =
                    double_ratchet.encrypt_message(message_content, &ad)?;

                Ok(Message::Regular(
                    self.x3dh.identity_key.public_key.clone(),
                    double_ratchet_message,
                ))
            }
            // This branch is the case in which we haven't received a response
            // so we continue to wrap DoubleRatchetMessages in X3DHMessages.
            // Note we *don't* run self.x3dh.derive_initial_keys because the
            // Double Ratchet protocol handles lost messages just fine.
            (Some(double_ratchet), Some((secret_key, ephemeral_public_key))) => {
                let serialized_message =
                    double_ratchet.encrypt_message_and_serialize(message_content, &ad)?;
                let x3dh_message = self.x3dh.construct_initial_message(
                    &serialized_message,
                    &secret_key,
                    &ephemeral_public_key,
                    ad,
                );

                self.unacknowledged_x3dh = Some((secret_key, ephemeral_public_key));
                Ok(Message::X3DH(x3dh_message))
            }
        }
    }

    // Attempting to decrypt a valid X3DH message will reset the
    // DoubleRatchetClient, so attempting to decrypt the same message multiple
    // times has the risk of making later messages undecipherable!
    // TODO: is it possible to prevent this at this layer in a nice way?
    pub fn attempt_message_decryption<R: CryptoRng + RngCore>(
        &mut self,
        csprng: &mut R,
        message: Message,
    ) -> Result<Vec<u8>, CryptoError> {
        match (message, self.double_ratchet.as_mut()) {
            // If we get a regular DoubleRatchetMessage without a
            // DoubleRatchetClient, the only thing we can do is reject it.
            (Message::Regular(_, _), None) => Err(CryptoError::UnreadableDoubleRatchetMessage),
            // When we get a valid X3DHMessage, we initialize or reset the
            // DoubleRatchetClient.
            (Message::X3DH(encrypted_message), _) => {
                let (secret_key, decrypted_message) = self.x3dh.decrypt_initial_message(
                    &encrypted_message,
                    &self.their_info,
                    &self.our_info,
                )?;

                let mut double_ratchet =
                    DoubleRatchetClient::respond(secret_key, &self.x3dh.prekey);
                let double_ratchet_message: DoubleRatchetMessage =
                    bincode::deserialize(&decrypted_message).map_err(|err| {
                        CryptoError::Deserialization("DoubleRatchetMessage".to_string(), *err)
                    })?;
                let ad = X3DHClient::build_associated_data(
                    // TODO: Is it correct here to use the identity_key
                    // provided in the X3DHMessage header?
                    &encrypted_message.identity_key,
                    &self.x3dh.identity_key.public_key,
                    &self.their_info,
                    &self.our_info,
                );

                let content = double_ratchet.attempt_message_decryption(
                    csprng,
                    &double_ratchet_message,
                    &ad,
                )?;

                self.double_ratchet = Some(double_ratchet);
                self.unacknowledged_x3dh = None;

                Ok(content)
            }
            (Message::Regular(their_identity_key, encrypted_message), Some(double_ratchet)) => {
                let ad = X3DHClient::build_associated_data(
                    &their_identity_key,
                    &self.x3dh.identity_key.public_key,
                    &self.their_info,
                    &self.our_info,
                );
                let content =
                    double_ratchet.attempt_message_decryption(csprng, &encrypted_message, &ad)?;
                self.unacknowledged_x3dh = None;
                Ok(content)
            }
        }
    }
}

#[cfg(test)]
pub mod common {
    use quickcheck::{Arbitrary, Gen};
    use rand::prelude::SliceRandom;

    #[derive(Debug, Clone)]
    pub enum Sender {
        Alice,
        Bob,
    }

    impl Arbitrary for Sender {
        fn arbitrary<G: Gen>(mut g: &mut G) -> Self {
            [Sender::Alice, Sender::Bob]
                .choose(&mut g)
                .expect("choose value")
                .clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Sender;
    use rand::rngs::OsRng;

    #[quickcheck]
    fn one_message_works(message_content: Vec<u8>) -> bool {
        let mut csprng = OsRng;
        let alice_info = b"alice";
        let bob_info = b"bob";

        let mut alice = Client::new(&mut csprng, alice_info, bob_info);
        let mut bob = Client::new(&mut csprng, bob_info, alice_info);

        let encrypted_message = alice
            .create_message(
                &mut csprng,
                &bob.x3dh.identity_key.public_key,
                &bob.x3dh.prekey.public_key,
                &message_content,
            )
            .expect("encryption should succeed");
        let decrypted_message = bob
            .attempt_message_decryption(&mut csprng, encrypted_message)
            .expect("decryption should succeed");

        message_content == decrypted_message
    }

    fn exchange_multiple_messages(
        message_content: &[u8],
        sender_order: &[(Sender, bool)],
    ) -> Vec<Option<Vec<u8>>> {
        // We use an empty message here, since the first message is already
        // covered by the one_message_works quickcheck test.
        let empty_message = Vec::new();

        let mut csprng = OsRng;
        let alice_info = b"alice";
        let bob_info = b"bob";

        let mut alice = Client::new(&mut csprng, alice_info, bob_info);
        let mut bob = Client::new(&mut csprng, bob_info, alice_info);

        let encrypted_message = alice
            .create_message(
                &mut csprng,
                &bob.x3dh.identity_key.public_key,
                &bob.x3dh.prekey.public_key,
                &empty_message,
            )
            .expect("encryption should succeed");
        let decrypted_message = bob
            .attempt_message_decryption(&mut csprng, encrypted_message)
            .expect("decryption should succeed");

        assert_eq!(empty_message, decrypted_message);

        let mut decrytion_results = Vec::new();
        for (sender, delivered) in sender_order.iter() {
            match sender {
                Sender::Alice => {
                    let encrypted_message = alice
                        .create_message(
                            &mut csprng,
                            &bob.x3dh.identity_key.public_key,
                            &bob.x3dh.prekey.public_key,
                            &message_content,
                        )
                        .expect("encryption should succeed");

                    if *delivered {
                        let decrypted_message =
                            bob.attempt_message_decryption(&mut csprng, encrypted_message);
                        decrytion_results.push(decrypted_message.ok());
                    } else {
                        decrytion_results.push(None);
                    }
                }
                Sender::Bob => {
                    let encrypted_message = bob
                        .create_message(
                            &mut csprng,
                            &alice.x3dh.identity_key.public_key,
                            &alice.x3dh.prekey.public_key,
                            &message_content,
                        )
                        .expect("encryption should succeed");

                    if *delivered {
                        let decrypted_message =
                            alice.attempt_message_decryption(&mut csprng, encrypted_message);
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
    fn multiple_messages_works(
        message_content: Vec<u8>,
        sender_order: Vec<(Sender, bool)>,
    ) -> bool {
        let results = exchange_multiple_messages(&message_content, &sender_order);
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
    fn test_case_1() {
        let message_content = Vec::new();
        let decrypted_messages = exchange_multiple_messages(
            &message_content,
            &[(Sender::Alice, false), (Sender::Bob, true)],
        );
        assert_eq!(decrypted_messages, [None, Some(message_content.clone())]);
    }

    #[test]
    fn test_case_2() {
        let message_content = Vec::new();
        let decrypted_messages = exchange_multiple_messages(
            &message_content,
            &[(Sender::Bob, false), (Sender::Bob, true)],
        );
        assert_eq!(decrypted_messages, [None, Some(message_content.clone())]);
    }

    #[test]
    #[ignore]
    fn test_async_x3dh_inconsistency() {
        let mut csprng = OsRng;
        let alice_info = b"alice";
        let bob_info = b"bob";

        let mut alice = Client::new(&mut csprng, alice_info, bob_info);
        let mut bob = Client::new(&mut csprng, bob_info, alice_info);

        // first, alice initiates conversation (ratchet A)
        let alice_x3dh = alice
            .create_message(
                &mut csprng,
                &bob.x3dh.identity_key.public_key,
                &bob.x3dh.prekey.public_key,
                b"alice X3DH",
            )
            .unwrap();

        // and encrypt a message using ratchet A
        let alice_msg1 = alice
            .create_message(
                &mut csprng,
                &bob.x3dh.identity_key.public_key,
                &bob.x3dh.prekey.public_key,
                b"alice DR1",
            )
            .unwrap();

        // although bob is trying to initiate too. This may lead to another ratchet B?
        let bob_x3dh = bob
            .create_message(
                &mut csprng,
                &alice.x3dh.identity_key.public_key,
                &alice.x3dh.prekey.public_key,
                b"bob X3DH",
            )
            .unwrap();

        // alice receives X3DH from bob, and switch to ratchet B (supposedly)?
        let bob_x3dh_received = alice
            .attempt_message_decryption(&mut csprng, bob_x3dh)
            .unwrap();
        assert_eq!(bob_x3dh_received, b"bob X3DH");

        // alice send another message encrypted by ratchet B.
        let alice_msg2 = alice
            .create_message(
                &mut csprng,
                &bob.x3dh.identity_key.public_key,
                &bob.x3dh.prekey.public_key,
                b"alice DR2",
            )
            .unwrap();

        // bob tries to decrypt them.
        // this message causes bob to throw away ratchet B and to use ratchet A?
        let alice_x3dh_received = bob
            .attempt_message_decryption(&mut csprng, alice_x3dh)
            .unwrap();
        assert_eq!(alice_x3dh_received, b"alice X3DH");

        // msg1 is encrypted by ratchet A, so bob succeeds to decrypt.
        let alice_msg1_received = bob
            .attempt_message_decryption(&mut csprng, alice_msg1)
            .unwrap();
        assert_eq!(alice_msg1_received, b"alice DR1");

        // but, msg2 is encrypted by ratchet B, and thus bob failed to decrypt.
        let ailce_msg2_received = bob
            .attempt_message_decryption(&mut csprng, alice_msg2)
            .unwrap();
        assert_eq!(ailce_msg2_received, b"alice DR2");
    }
}
