use mizu_sqlite::MizuConnection;
use tezos_interface::Tezos;
use mizu_crypto::x3dh::X3DHClient;
use mizu_crypto::Client;
use bincode::{deserialize, serialize};
use thiserror::Error;
use mizu_sqlite::{identity::Identity, contact::Contact, message::Message};
use std::fmt::{self, Display, Debug};
use rand::{RngCore, CryptoRng};
use chrono::naive::NaiveDateTime;

type DieselError = diesel::result::Error;

#[derive(Debug, Error)]
pub enum DriverError<RE: Debug + Display, WE: Debug + Display> {
    #[error("failed to parse command: {0}")]
    ParseFail(String),
    #[error("something not found")]
    NotFound,
    #[error("persistency layer: {0}")]
    UserData(DieselError),
    #[error("Tezos read: {0}")]
    TezosRead(RE),
    #[error("Tezos write: {0}")]
    TezosWrite(WE),
    #[error("Invalid X3DH: {0}")]
    InvalidX3DH(bincode::Error),
    #[error("Invalid Client: {0}")]
    InvalidClient(bincode::Error),
}

// All states needed to run protocols are saved to a SQLite database and retrieved on demand.
pub struct Driver<T> {
    conn: MizuConnection,
    tezos: T,
}

pub type DriverResult<T, A> = Result<A, DriverError<<T as Tezos>::ReadError, <T as Tezos>::WriteError>>;

// TODO: better name
struct ClientAndTimestamp {
    client: Client,
    latest_message_timestamp: Option<NaiveDateTime>,
}

impl<T> Driver<T>
where
    T: Tezos,
    T::ReadError: Debug + Display,
    T::WriteError: Debug + Display,
{
    pub fn new(conn: MizuConnection, tezos: T) -> Self {
        Self {
            conn,
            tezos,
        }
    }

    pub fn list_identities(&self) -> DriverResult<T, Vec<Identity>> {
        self.conn.list_identities().map_err(DriverError::UserData)
    }

    pub fn list_contacts(&self) -> DriverResult<T, Vec<Contact>> {
        self.conn.list_contacts().map_err(DriverError::UserData)
    }

    pub fn list_messages(&self, our_identity_id: i32, their_contact_id: i32) -> DriverResult<T, Vec<Message>> {
        self.conn.find_messages(our_identity_id, their_contact_id).map_err(DriverError::UserData)
    }

    pub fn generate_identity<R: RngCore + CryptoRng>(&self, rng: &mut R, name: &str) -> DriverResult<T, ()> {
        let x3dh = X3DHClient::new(rng);
        self.conn.create_identity(name, &x3dh).map_err(DriverError::UserData)
    }

    /// publish local identity to Tezos
    pub fn publish_identity(&self, identity_id: i32) -> DriverResult<T, ()> {
        use DriverError::*;

        let identity = self.conn.find_identity(identity_id).map_err(UserData)?;
        let x3dh: X3DHClient = deserialize(&identity.x3dh_client).map_err(InvalidX3DH)?;
        self.tezos.register(
            Some(x3dh.identity_key.public_key.0.as_bytes()),
            x3dh.prekey.public_key.0.as_bytes(),
        ).map_err(TezosWrite)
    }

    pub fn add_contact(&self, name: &str, address: &str) -> DriverResult<T, ()> {
        self.conn.create_contact(name, address).map_err(DriverError::UserData)
    }

    pub fn find_user(&self, address: &str) -> DriverResult<T, Option<tezos_interface::UserData>> {
        self.tezos.retrieve_user_data(address).map_err(DriverError::TezosRead)
    }

    fn find_client(&self, our_identity_id: i32, their_contact_id: i32) -> DriverResult<T, Option<ClientAndTimestamp>> {
        use DriverError::*;
        
        self.conn.find_client(our_identity_id, their_contact_id).map_err(UserData)?
            .map(|client| Ok(ClientAndTimestamp {
                client: deserialize(&client.client_data).map_err(InvalidClient)?,
                latest_message_timestamp: client.latest_message_timestamp,
            }))
            .transpose()
    }

    // TODO: what if posting to Tezos succeeds but saving to SQLite fails?
}