#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

pub mod double_ratchet;
pub mod error;
pub mod keys;
pub mod x3dh;

use double_ratchet::{DoubleRatchetClient, DoubleRatchetMessage};
use error::CryptoError;
use keys::{IdentityPublicKey, PrekeyPublicKey};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use x3dh::{X3DHClient, X3DHMessage};

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
enum Message {
    X3DH(X3DHMessage),
    Regular(IdentityPublicKey, DoubleRatchetMessage),
}

// TODO: What happens when each side creates and sends a X3DH message for the other?
// TODO: there needs to be some way to persist this to disk
struct Client {
    x3dh: X3DHClient,
    double_ratchet: Option<DoubleRatchetClient>,
    our_info: Vec<u8>,
    their_info: Vec<u8>,
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
            our_info: our_info.iter().cloned().collect(),
            their_info: their_info.iter().cloned().collect(),
        }
    }

    pub fn create_x3dh_message<R: CryptoRng + RngCore>(
        &mut self,
        csprng: &mut R,
        recipient_identity_key: &IdentityPublicKey,
        recipient_prekey: &PrekeyPublicKey,
        message_content: &[u8],
    ) -> Result<Message, CryptoError> {
        let (secret_key, ephemeral_public_key) =
            self.x3dh
                .derive_initial_keys(csprng, recipient_identity_key, recipient_prekey);
        let ad = X3DHClient::build_associated_data(
            &self.x3dh.identity_key.public_key,
            &recipient_identity_key,
            &self.our_info,
            &self.their_info,
        );

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
        Ok(Message::X3DH(x3dh_message))
    }

    pub fn create_regular_message(
        &mut self,
        recipient_identity_key: &IdentityPublicKey,
        message_content: &[u8],
    ) -> Result<Message, CryptoError> {
        let ad = X3DHClient::build_associated_data(
            &self.x3dh.identity_key.public_key,
            &recipient_identity_key,
            &self.our_info,
            &self.their_info,
        );

        let double_ratchet_message = self
            .double_ratchet
            .as_mut()
            .expect("DoubleRatchetClient should already have been initialized at this point")
            .encrypt_message(message_content, &ad)?;

        Ok(Message::Regular(
            self.x3dh.identity_key.public_key.clone(),
            double_ratchet_message,
        ))
    }

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

                Ok(content)
            }
            (Message::Regular(their_identity_key, encrypted_message), Some(double_ratchet)) => {
                let ad = X3DHClient::build_associated_data(
                    &their_identity_key,
                    &self.x3dh.identity_key.public_key,
                    &self.their_info,
                    &self.our_info,
                );
                double_ratchet.attempt_message_decryption(csprng, &encrypted_message, &ad)
            }
        }
    }
}

//#[cfg(test)]
//mod tests {
//    #[quickcheck]
//    fn x3dh_with_double_ratchet(message_content: Vec<u8>) -> bool {
//    }
//}
