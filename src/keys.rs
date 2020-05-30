use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::convert::TryInto;
use x25519_dalek::*;

// X3DH

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityPublicKey(pub PublicKey);
pub struct IdentityKeyPair {
    private_key: StaticSecret,
    pub public_key: IdentityPublicKey,
}

impl IdentityKeyPair {
    pub fn new<R: CryptoRng + RngCore>(mut csprng: &mut R) -> IdentityKeyPair {
        let private_key = StaticSecret::new(&mut csprng);
        let public_key = IdentityPublicKey(PublicKey::from(&private_key));
        IdentityKeyPair {
            private_key: private_key,
            public_key: public_key,
        }
    }

    pub fn dh_pk(&self, public_key: &PrekeyPublicKey) -> SharedSecret {
        self.private_key.diffie_hellman(&public_key.0)
    }

    pub fn dh_ek(&self, public_key: &EphemeralPublicKey) -> SharedSecret {
        self.private_key.diffie_hellman(&public_key.0)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrekeyPublicKey(pub PublicKey);
pub struct PrekeyKeyPair {
    // While the prekey keypair has a shorter lifespan than that of the
    // identity keypair, its lifespan is still is on the order of days or
    // weeks at the shortest, so must be serializable (i.e. implemented as
    // StaticSecret instead of EphemeralSecret).
    private_key: StaticSecret,
    pub public_key: PrekeyPublicKey,
}

impl PrekeyKeyPair {
    pub fn new<R: CryptoRng + RngCore>(mut csprng: &mut R) -> PrekeyKeyPair {
        let private_key = StaticSecret::new(&mut csprng);
        let public_key = PrekeyPublicKey(PublicKey::from(&private_key));
        PrekeyKeyPair {
            private_key: private_key,
            public_key: public_key,
        }
    }

    pub fn dh(&self, public_key: &PublicKey) -> SharedSecret {
        self.private_key.diffie_hellman(public_key)
    }

    // TODO: depending on how Double Ratchet works, it may be possible to
    // change this to move self instead of borrowing it in order to prevent
    // key reuse. Should investigate.
    pub fn convert_to_ratchet_keypair(&self) -> RatchetKeyPair {
        RatchetKeyPair {
            private_key: self.private_key.clone(),
            public_key: RatchetPublicKey(self.public_key.0),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EphemeralPublicKey(pub PublicKey);

// Double Ratchet

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RatchetPublicKey(pub PublicKey);
pub struct RatchetKeyPair {
    // Similar situation as PrekeyKeyPair's StaticSecret.
    private_key: StaticSecret,
    pub public_key: RatchetPublicKey,
}

impl RatchetKeyPair {
    pub fn new<R: CryptoRng + RngCore>(mut csprng: &mut R) -> RatchetKeyPair {
        let private_key = StaticSecret::new(&mut csprng);
        let public_key = RatchetPublicKey(PublicKey::from(&private_key));
        RatchetKeyPair {
            private_key: private_key,
            public_key: public_key,
        }
    }

    pub fn dh(&self, public_key: &RatchetPublicKey) -> SharedSecret {
        self.private_key.diffie_hellman(&public_key.0)
    }
}

pub struct RootKey(pub [u8; 32]);

static INFO_RK: &'static [u8; 19] = b"MizuProtocolRootKey";

impl RootKey {
    // update RootKey and return the next ChainKey
    pub fn kdf(&mut self, shared_secret: SharedSecret) -> ChainKey {
        let h = Hkdf::<Sha256>::new(Some(&self.0), shared_secret.as_bytes());
        let mut rk = [0u8; 32];
        let mut ck = [0u8; 32];
        h.expand(INFO_RK, &mut rk).unwrap();
        h.expand(INFO_RK, &mut ck).unwrap();

        self.0 = rk;
        ChainKey(ck)
    }
}

pub struct ChainKey([u8; 32]);
// Key material for the nonce used in AEAD is bundled along with message key
pub struct MessageKey(pub [u8; 32], pub [u8; 32]);

impl ChainKey {
    fn hmac(key: &[u8], input: &[u8]) -> [u8; 32] {
        let mut mac = Hmac::<Sha256>::new_varkey(key).unwrap();
        mac.input(input);
        mac.result().code().as_slice().try_into().unwrap()
    }

    // update ChainKey and return the next MessageKey
    pub fn kdf(&mut self) -> MessageKey {
        let mk = ChainKey::hmac(&self.0, &[1]);

        // While deriving the nonce from the chain key is suggested by
        // the Double Ratchet spec, how it should be derived is not specified.
        // Here, we do it by passing in a different input to the HMAC keyed by
        // the chain key.
        let nonce = ChainKey::hmac(&self.0, &[3]);

        self.0 = ChainKey::hmac(&self.0, &[2]);
        MessageKey(mk, nonce)
    }
}
