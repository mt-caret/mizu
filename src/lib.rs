extern crate aes_gcm;
extern crate bincode;
extern crate hkdf;
extern crate rand;
extern crate serde;
extern crate sha2;
extern crate x25519_dalek;

pub mod keys;

use aes_gcm::aead::{generic_array::GenericArray, Aead, NewAead, Payload};
use aes_gcm::Aes256Gcm;
use hkdf::Hkdf;
use keys::{
    EphemeralPublicKey, IdentityKeyPair, IdentityPublicKey, PrekeyKeyPair, PrekeyPublicKey,
};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::*;

static INFO: &'static [u8; 12] = b"MizuProtocol";

pub struct X3DHClient {
    identity_key: IdentityKeyPair,
    prekey: PrekeyKeyPair,
}

pub struct X3DHSecretKey([u8; 32]);

#[derive(Serialize, Deserialize, Debug)]
pub struct InitialMessage {
    // TODO: identity_key seems redundant in our case since it's already
    // published in user_data of the sender, which should be known by the
    // recipient at this point. This could save the Mizu client the work of
    // going through past transactions (this may even be impossible if Mizu is
    // operating in delegated mode) if the identity_key has been changed in
    // the meantime, though.
    //
    // We purposefully do not identify which prekey of the recipient was used
    // in the message, since all participants can then trivially identify the
    // recipient by checking all users' prekeys for a match. Message recipients
    // should instead keep the two most recent prekeys along with when rotation
    // occured and use the appropriate prekey based on the timestamp of the
    // message.
    identity_key: IdentityPublicKey,
    ephemeral_key: EphemeralPublicKey,
    ciphertext: Vec<u8>,
}

impl X3DHClient {
    pub fn new<R: CryptoRng + RngCore>(mut csprng: &mut R) -> X3DHClient {
        let identity_key = IdentityKeyPair::new(&mut csprng);
        let prekey = PrekeyKeyPair::new(&mut csprng);
        X3DHClient {
            identity_key: identity_key,
            prekey: prekey,
        }

        // TODO: publish keys to smart contract?
    }

    fn kdf(input: &[u8]) -> [[u8; 32]; 3] {
        // We prepend 32 bytes of 0xff here, per the specification in X3DH.
        let ikm = [&[0xff; 32], input].concat();

        // The salt is set to None, which is then automatically zeroed out.
        let h = Hkdf::<Sha256>::new(None, &ikm);
        let mut okm0 = [0u8; 32];
        let mut okm1 = [0u8; 32];
        let mut okm2 = [0u8; 32];

        h.expand(INFO, &mut okm0).unwrap();
        h.expand(INFO, &mut okm1).unwrap();
        h.expand(INFO, &mut okm2).unwrap();
        [okm0, okm1, okm2]
    }

    pub fn derive_initial_keys<R: CryptoRng + RngCore>(
        &self,
        mut csprng: &mut R,
        ik: &IdentityPublicKey,
        pk: &PrekeyPublicKey,
    ) -> (X3DHSecretKey, EphemeralPublicKey) {
        // Note usage of StaticSecret while it seems like EphemeralSecret
        // should be used. This is because EphemeralSecret does not implement
        // the Copy/Clone trait and EphemeralSecret::diffie_hellman does not
        // borrow the private key to prevent reuse. This API is adequate for
        // normal usage but since we reuse the same secret for dh2 and dh3,
        // we cannot use EphemeralSecret.
        let ephemeral_private_key = StaticSecret::new(&mut csprng);
        let ephemeral_public_key = PublicKey::from(&ephemeral_private_key);

        let dh1 = *self.identity_key.dh(&pk).as_bytes();
        let dh2 = *ephemeral_private_key.diffie_hellman(&ik.0).as_bytes();
        let dh3 = *ephemeral_private_key.diffie_hellman(&pk.0).as_bytes();
        let kdf_input = [dh1, dh2, dh3].concat();
        let [secret_key, _, _] = X3DHClient::kdf(&kdf_input);

        (
            X3DHSecretKey(secret_key),
            EphemeralPublicKey(ephemeral_public_key),
        )
    }

    pub fn build_associated_data(
        sender_key: &IdentityPublicKey,
        receiver_key: &IdentityPublicKey,
        sender_info: &[u8],
        receiver_info: &[u8],
    ) -> Vec<u8> {
        [
            sender_key.0.as_bytes(),
            receiver_key.0.as_bytes(),
            sender_info,
            receiver_info,
        ]
        .concat()
    }

    pub fn construct_initial_message(
        &self,
        content: &[u8],
        secret_key: X3DHSecretKey,
        ephemeral_key: &EphemeralPublicKey,
        receiver_key: &IdentityPublicKey,
        sender_info: &[u8],
        receiver_info: &[u8],
    ) -> Vec<u8> {
        let [key, _, nonce_base] = X3DHClient::kdf(&secret_key.0);
        let key = GenericArray::from_slice(&key);
        let nonce = GenericArray::from_slice(&nonce_base[0..12]);
        let aad = X3DHClient::build_associated_data(
            &self.identity_key.public_key,
            receiver_key,
            sender_info,
            receiver_info,
        );
        let payload = Payload {
            msg: content,
            aad: &aad,
        };
        let cipher = Aes256Gcm::new(*key);
        let ciphertext = cipher.encrypt(&nonce, payload).unwrap();

        let message = InitialMessage {
            identity_key: self.identity_key.public_key.clone(),
            ephemeral_key: ephemeral_key.clone(),
            ciphertext: ciphertext,
        };

        // TODO: We use serde and bincode to serialize the message.
        // This creates a potential issue: is it possible to differentiate
        // between types of messages (i.e. does bincode leave some sort of
        // marker so the type of serialized data can be identified)?
        // If it is, then anybody can see which types of messages are being
        // sent, which when combined with message size, can be considered to
        // be a case of nontrivial metadata leakage.
        bincode::serialize(&message).unwrap()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
