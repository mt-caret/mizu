use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use x25519_dalek::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityPublicKey(pub PublicKey);
pub struct IdentityKeyPair {
    private_key: StaticSecret,
    pub public_key: IdentityPublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrekeyPublicKey(pub PublicKey);
pub struct PrekeyKeyPair {
    // While the prekey keypair has a shorter lifespan than that of the
    // identity keypair, its lifespan is still is on the order of days or
    // weeks at the shortest, so must be serializable (i.e. implemented as
    // StaticSecret instead of EphemeralSecret).
    private_key: StaticSecret,
    public_key: PrekeyPublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EphemeralPublicKey(pub PublicKey);

impl IdentityKeyPair {
    pub fn new<R: CryptoRng + RngCore>(mut csprng: &mut R) -> IdentityKeyPair {
        let private_key = StaticSecret::new(&mut csprng);
        let public_key = IdentityPublicKey(PublicKey::from(&private_key));
        IdentityKeyPair {
            private_key: private_key,
            public_key: public_key,
        }
    }

    pub fn dh(&self, public_key: &PrekeyPublicKey) -> SharedSecret {
        self.private_key.diffie_hellman(&public_key.0)
    }
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
}
