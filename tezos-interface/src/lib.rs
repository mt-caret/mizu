use chrono::naive::NaiveDateTime;

#[derive(Debug)]
pub struct Message {
    pub content: Vec<u8>,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug)]
pub struct UserData {
    pub identity_key: Vec<u8>,
    pub prekey: Vec<u8>,
    pub postal_box: Vec<Message>,
    pub pokes: Vec<Vec<u8>>,
}

pub trait Tezos {
    type ReadError;
    type WriteError;

    // Read
    /// Returns Tezos address.
    fn address(&self) -> &[u8];
    /// Retrieve Mizu user data associated with the specified address in Tezos.
    fn retrieve_user_data(&self, address: &[u8]) -> Result<Option<UserData>, Self::ReadError>;

    // Update
    // TODO: I don't think double slices is a good interface, as we can't pass &[Vec<u8>] for example.
    // The best I came up with is taking `A: IntoIter<&[u8]>`, but it will break object safety...
    // TODO: remove should take `BigUint`s
    fn post(&self, add: &[&[u8]], remove: &[&usize]) -> Result<(), Self::WriteError>;
    fn poke(&self, target_address: &[u8], data: &[u8]) -> Result<(), Self::WriteError>;
    fn register(&self, identity_key: Option<&[u8]>, prekey: &[u8]) -> Result<(), Self::WriteError>;
}
