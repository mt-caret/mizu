//! TODO: error handling

#[derive(Debug)]
pub struct Message {
    pub content: Vec<u8>,
    pub timestamp: String,
}

#[derive(Debug)]
pub struct UserData {
    pub identity_key: Vec<u8>,
    pub prekey: Vec<u8>,
    pub postal_box: Vec<Message>,
    pub pokes: Vec<Vec<u8>>,
}

pub trait Tezos {
    // Read
    fn retrieve_user_data(&self, address: &[u8]) -> Option<UserData>;

    // Update
    // TODO: I don't think double slices is a good interface, as we can't pass &[Vec<u8>] for example.
    // The best I came up with is taking `A: IntoIter<&[u8]>`, but it will break object safety...
    fn post(&self, sender_address: &[u8], add: &[&[u8]], remove: &[&usize]);
    fn poke(&self, target_address: &[u8], data: &[u8]);
    fn register(&self, sender_address: &[u8], identity_key: Option<&[u8]>, prekey: &[u8]);
}
