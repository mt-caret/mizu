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
