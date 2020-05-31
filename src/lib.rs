extern crate aes_gcm;
extern crate bincode;
extern crate hkdf;
extern crate hmac;
extern crate rand;
extern crate serde;
extern crate sha2;
extern crate x25519_dalek;

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

pub mod double_ratchet;
pub mod keys;
pub mod x3dh;

//use serde::{Deserialize, Serialize};
//
//#[derive(Serialize, Deserialize, Debug)]
//struct Message {
//    header: Option<x3dh::InitialMessage>,
//    body: double_ratchet::DoubleRatchetMessage,
//}
//
//#[cfg(test)]
//mod tests {
//    #[quickcheck]
//    fn x3dh_with_double_ratchet(message_content: Vec<u8>) -> bool {
//    }
//}
