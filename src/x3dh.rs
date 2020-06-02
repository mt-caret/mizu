use crate::error::CryptoError;
use crate::keys::{
    EphemeralPublicKey, IdentityKeyPair, IdentityPublicKey, PrekeyKeyPair, PrekeyPublicKey,
};
use aes_gcm::aead::{generic_array::GenericArray, Aead, NewAead, Payload};
use aes_gcm::Aes256Gcm;
use hkdf::Hkdf;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::*;

static INFO: &[u8; 12] = b"MizuProtocol";

pub struct X3DHClient {
    // We omit the one-time prekey here, since we trust the Tezos blockchain
    // to not "replay" messages.
    pub identity_key: IdentityKeyPair,
    pub prekey: PrekeyKeyPair,
}

#[derive(Clone)]
pub struct X3DHSecretKey(pub [u8; 32]);

#[derive(Serialize, Deserialize, Debug)]
pub struct X3DHMessage {
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
    pub identity_key: IdentityPublicKey,
    ephemeral_key: EphemeralPublicKey,
    ciphertext: Vec<u8>,
}

pub struct X3DHAD(pub Vec<u8>);

impl X3DHClient {
    pub fn new<R: CryptoRng + RngCore>(csprng: &mut R) -> X3DHClient {
        let identity_key = IdentityKeyPair::new(csprng);
        let prekey = PrekeyKeyPair::new(csprng);
        X3DHClient {
            identity_key,
            prekey,
        }

        // TODO: publish keys to smart contract?
    }

    fn kdf(input: &[u8]) -> [[u8; 32]; 3] {
        // We prepend 32 bytes of 0xff here, per the X3DH spec.
        let ikm = [&[0xff; 32], input].concat();

        // The salt is set to None, which is then automatically zeroed out.
        let h = Hkdf::<Sha256>::new(None, &ikm);
        let mut okm0 = [0u8; 32];
        let mut okm1 = [0u8; 32];
        let mut okm2 = [0u8; 32];

        // The underlying implementation of HKDF only returns Err when
        // okm is larger than 255 times the size of prk
        // (https://docs.rs/hkdf/0.8.0/src/hkdf/hkdf.rs.html#102-129).
        // okm is much smaller, so it is safe to unwrap here.
        h.expand(INFO, &mut okm0).unwrap();
        h.expand(INFO, &mut okm1).unwrap();
        h.expand(INFO, &mut okm2).unwrap();
        [okm0, okm1, okm2]
    }

    pub fn derive_initial_keys<R: CryptoRng + RngCore>(
        &self,
        csprng: &mut R,
        ik: &IdentityPublicKey,
        pk: &PrekeyPublicKey,
    ) -> (X3DHSecretKey, EphemeralPublicKey) {
        // Note usage of StaticSecret while it seems like EphemeralSecret
        // should be used. This is because EphemeralSecret does not implement
        // the Copy/Clone trait and EphemeralSecret::diffie_hellman does not
        // borrow the private key to prevent reuse. This API is adequate for
        // normal usage but since we reuse the same secret for dh2 and dh3,
        // we cannot use EphemeralSecret.
        let ephemeral_private_key = StaticSecret::new(csprng);
        let ephemeral_public_key = PublicKey::from(&ephemeral_private_key);

        let dh1 = *self.identity_key.dh_pk(&pk).as_bytes();
        let dh2 = *ephemeral_private_key.diffie_hellman(&ik.0).as_bytes();
        let dh3 = *ephemeral_private_key.diffie_hellman(&pk.0).as_bytes();
        let kdf_input = [dh1, dh2, dh3].concat();
        let [secret_key, _, _] = X3DHClient::kdf(&kdf_input);

        (
            X3DHSecretKey(secret_key),
            EphemeralPublicKey(ephemeral_public_key),
        )
    }

    // sender_info and receiver_info passed here *must* include information of
    // the Tezos addresses of the sender and receiver in order to prevent
    // "unknown key share" attacks. See X3DH spec section 4.8
    // (Identity binding).
    pub fn build_associated_data(
        sender_key: &IdentityPublicKey,
        receiver_key: &IdentityPublicKey,
        sender_info: &[u8],
        receiver_info: &[u8],
    ) -> X3DHAD {
        X3DHAD(
            [
                sender_key.0.as_bytes(),
                receiver_key.0.as_bytes(),
                sender_info,
                receiver_info,
            ]
            .concat(),
        )
    }

    pub fn construct_initial_message(
        &self,
        content: &[u8],
        secret_key: &X3DHSecretKey,
        ephemeral_key: &EphemeralPublicKey,
        associated_data: X3DHAD,
    ) -> X3DHMessage {
        // TODO: I think running the secret through the kdf and using the
        // outputs this way is valid; should check libsignal sources and
        // mimic what they do.
        let [key, _, nonce_base] = X3DHClient::kdf(&secret_key.0);
        let key = GenericArray::from_slice(&key);
        let nonce = GenericArray::from_slice(&nonce_base[0..12]);
        let payload = Payload {
            msg: content,
            aad: &associated_data.0,
        };

        // One pitfall when using AES in GCM mode is nonce reuse;
        // we can be reasonably sure this will not happen as the nonce
        // is derived from a KDF which in turn is th result of contains
        // input from an ephemeral keypair that we have randomly generated
        // just before.
        let cipher = Aes256Gcm::new(*key);
        let ciphertext = cipher.encrypt(&nonce, payload).unwrap();

        X3DHMessage {
            identity_key: self.identity_key.public_key.clone(),
            ephemeral_key: ephemeral_key.clone(),
            ciphertext,
        }
    }

    // TODO: Is it safe to blindly trust identity_key provided in this
    // message, or does it open us to attacks?
    pub fn decrypt_initial_message(
        &self,
        message: &X3DHMessage,
        sender_info: &[u8],
        receiver_info: &[u8],
    ) -> Result<(X3DHSecretKey, Vec<u8>), CryptoError> {
        let dh1 = *self.prekey.dh(&message.identity_key.0).as_bytes();
        let dh2 = *self.identity_key.dh_ek(&message.ephemeral_key).as_bytes();
        let dh3 = *self.prekey.dh(&message.ephemeral_key.0).as_bytes();
        let kdf_input = [dh1, dh2, dh3].concat();
        let [secret_key, _, _] = X3DHClient::kdf(&kdf_input);

        let [key, _, nonce_base] = X3DHClient::kdf(&secret_key);
        let key = GenericArray::from_slice(&key);
        let nonce = GenericArray::from_slice(&nonce_base[0..12]);
        let associated_data = X3DHClient::build_associated_data(
            &message.identity_key,
            &self.identity_key.public_key,
            sender_info,
            receiver_info,
        );
        let payload = Payload {
            msg: &message.ciphertext,
            aad: &associated_data.0,
        };
        let cipher = Aes256Gcm::new(*key);
        let plaintext = cipher
            .decrypt(&nonce, payload)
            .map_err(|_| CryptoError::AEADDecryption("InitialMessage".to_string()))?;

        Ok((X3DHSecretKey(secret_key), plaintext))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[quickcheck]
    fn x3dh_key_agreement_works(message_content: Vec<u8>) -> bool {
        let mut csprng = OsRng;
        let alice = X3DHClient::new(&mut csprng);
        let bob = X3DHClient::new(&mut csprng);

        // We assume here that bob's public keys are published somewhere,
        // and have been obtained in some way.
        let (alice_sk, alice_ek) = alice.derive_initial_keys(
            &mut csprng,
            &bob.identity_key.public_key,
            &bob.prekey.public_key,
        );
        let sender_info = b"alice";
        let receiver_info = b"bob";
        let associated_data = X3DHClient::build_associated_data(
            &alice.identity_key.public_key,
            &bob.identity_key.public_key,
            sender_info,
            receiver_info,
        );
        let encrypted_message = alice.construct_initial_message(
            &message_content,
            &alice_sk,
            &alice_ek,
            associated_data,
        );

        // Bob then gets an encrypted message, and proceeds to derive the
        // secret key and decrypt it.
        let (bob_sk, decrypted_message) = bob
            .decrypt_initial_message(&encrypted_message, sender_info, receiver_info)
            .unwrap();

        // If X3DH is implemented correctly, both Alice and Bob should end up
        // with the same secret key and the decrypted message should match
        // the original message.
        alice_sk.0 == bob_sk.0 && message_content == decrypted_message
    }

    fn create_random_message<R: CryptoRng + RngCore>(csprng: &mut R, junk: Vec<u8>) -> X3DHMessage {
        let identity_key = IdentityKeyPair::new(csprng).public_key;
        let ephemeral_key = EphemeralPublicKey(PublicKey::from(&StaticSecret::new(csprng)));
        X3DHMessage {
            identity_key,
            ephemeral_key,
            ciphertext: junk,
        }
    }

    // Let's say Mallory sends Bob a bunch of junk. Can Bob gracefully handle
    // this?
    #[quickcheck]
    fn x3dh_handles_failures_gracefully(junk: Vec<u8>) -> bool {
        let mut csprng = OsRng;
        let bob = X3DHClient::new(&mut csprng);

        let sender_info = b"mallory";
        let receiver_info = b"bob";

        let junk = create_random_message(&mut csprng, junk);
        bob.decrypt_initial_message(&junk, sender_info, receiver_info)
            .is_err()
    }
}
