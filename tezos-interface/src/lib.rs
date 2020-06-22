use chrono::naive::NaiveDateTime;
use std::fmt::{Debug, Display};

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
    type ReadError: Display + Debug;
    type WriteError: Debug + Display;

    // Read
    /// Returns Tezos address.
    fn address(&self) -> &str;
    /// Retrieve Mizu user data associated with the specified address in Tezos.
    fn retrieve_user_data(&self, address: &str) -> Result<Option<UserData>, Self::ReadError>;

    // Update
    // TODO: I don't think double slices is a good interface, as we can't pass &[Vec<u8>] for example.
    // The best I came up with is taking `A: IntoIter<&[u8]>`, but it will break object safety...
    // TODO: remove should take `BigUint`s
    fn post(&self, add: &[&[u8]], remove: &[&usize]) -> Result<(), Self::WriteError>;
    fn poke(&self, target_address: &str, data: &[u8]) -> Result<(), Self::WriteError>;
    fn register(&self, identity_key: Option<&[u8]>, prekey: &[u8]) -> Result<(), Self::WriteError>;
}

impl<'a, T: Tezos> Tezos for &'a T {
    type ReadError = T::ReadError;
    type WriteError = T::WriteError;

    fn address(&self) -> &str {
        (**self).address()
    }

    fn retrieve_user_data(&self, address: &str) -> Result<Option<UserData>, Self::ReadError> {
        (**self).retrieve_user_data(address)
    }

    fn post(&self, add: &[&[u8]], remove: &[&usize]) -> Result<(), Self::WriteError> {
        (**self).post(add, remove)
    }

    fn poke(&self, target_address: &str, data: &[u8]) -> Result<(), Self::WriteError> {
        (**self).poke(target_address, data)
    }

    fn register(&self, identity_key: Option<&[u8]>, prekey: &[u8]) -> Result<(), Self::WriteError> {
        (**self).register(identity_key, prekey)
    }
}

impl<T: Tezos> Tezos for Box<T> {
    type ReadError = T::ReadError;
    type WriteError = T::WriteError;

    fn address(&self) -> &str {
        (**self).address()
    }

    fn retrieve_user_data(&self, address: &str) -> Result<Option<UserData>, Self::ReadError> {
        (**self).retrieve_user_data(address)
    }

    fn post(&self, add: &[&[u8]], remove: &[&usize]) -> Result<(), Self::WriteError> {
        (**self).post(add, remove)
    }

    fn poke(&self, target_address: &str, data: &[u8]) -> Result<(), Self::WriteError> {
        (**self).poke(target_address, data)
    }

    fn register(&self, identity_key: Option<&[u8]>, prekey: &[u8]) -> Result<(), Self::WriteError> {
        (**self).register(identity_key, prekey)
    }
}

impl<T: Tezos> Tezos for std::sync::Arc<T> {
    type ReadError = T::ReadError;
    type WriteError = T::WriteError;

    fn address(&self) -> &str {
        (**self).address()
    }

    fn retrieve_user_data(&self, address: &str) -> Result<Option<UserData>, Self::ReadError> {
        (**self).retrieve_user_data(address)
    }

    fn post(&self, add: &[&[u8]], remove: &[&usize]) -> Result<(), Self::WriteError> {
        (**self).post(add, remove)
    }

    fn poke(&self, target_address: &str, data: &[u8]) -> Result<(), Self::WriteError> {
        (**self).poke(target_address, data)
    }

    fn register(&self, identity_key: Option<&[u8]>, prekey: &[u8]) -> Result<(), Self::WriteError> {
        (**self).register(identity_key, prekey)
    }
}
