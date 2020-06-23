use chrono::naive::NaiveDateTime;
use std::error::Error;
use std::fmt;

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

struct Boxed<T>(T);
// One of the most poor parts in Rust :(
#[derive(Debug)]
pub struct BoxedError(pub Box<dyn Error + Send + Sync + 'static>);

pub fn into_boxed_error<E: Error + Send + Sync + 'static>(error: E) -> BoxedError {
    BoxedError(Box::new(error))
}

impl fmt::Display for BoxedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for BoxedError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

pub type BoxedTezos<'a> = Box<dyn Tezos<ReadError = BoxedError, WriteError = BoxedError> + 'a>;

pub trait Tezos {
    type ReadError: Error + Send + Sync + 'static;
    type WriteError: Error + Send + Sync + 'static;

    fn boxed<'a>(self) -> BoxedTezos<'a>
    where
        Self: Sized + 'a,
    {
        Box::new(Boxed(self))
    }

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

impl<T: Tezos> Tezos for Boxed<T> {
    type ReadError = BoxedError;
    type WriteError = BoxedError;

    fn address(&self) -> &str {
        self.0.address()
    }

    fn retrieve_user_data(&self, address: &str) -> Result<Option<UserData>, Self::ReadError> {
        self.0.retrieve_user_data(address).map_err(into_boxed_error)
    }

    fn post(&self, add: &[&[u8]], remove: &[&usize]) -> Result<(), Self::WriteError> {
        self.0.post(add, remove).map_err(into_boxed_error)
    }

    fn poke(&self, target_address: &str, data: &[u8]) -> Result<(), Self::WriteError> {
        self.0.poke(target_address, data).map_err(into_boxed_error)
    }

    fn register(&self, identity_key: Option<&[u8]>, prekey: &[u8]) -> Result<(), Self::WriteError> {
        self.0
            .register(identity_key, prekey)
            .map_err(into_boxed_error)
    }
}
