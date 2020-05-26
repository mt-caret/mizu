extern crate hkdf;
extern crate rand;
extern crate sha2;
extern crate x25519_dalek;

pub mod keys;

use hkdf::Hkdf;
use keys::{IdentityKeyPair, IdentityPublicKey, PrekeyKeyPair, PrekeyPublicKey};
use rand::{CryptoRng, RngCore};
use sha2::Sha256;
use x25519_dalek::*;

static INFO: &'static [u8; 12] = b"MizuProtocol";

pub struct X3DHClient {
    identity_key: IdentityKeyPair,
    prekey: PrekeyKeyPair,
}

pub struct X3DHSecretKey([u8; 32]);

impl X3DHClient {
    pub fn new<R: CryptoRng + RngCore>(mut csprng: &mut R) -> X3DHClient {
        let identity_key = IdentityKeyPair::new(&mut csprng);
        let prekey = PrekeyKeyPair::new(&mut csprng);
        X3DHClient {
            identity_key: identity_key,
            prekey: prekey,
        }
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

    pub fn derive_initial_sending_key<R: CryptoRng + RngCore>(
        &self,
        mut csprng: &mut R,
        ik: &IdentityPublicKey,
        pk: &PrekeyPublicKey,
    ) -> X3DHSecretKey {
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

        X3DHSecretKey(secret_key)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
