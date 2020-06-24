use bincode::{deserialize, serialize};
use chrono::naive::NaiveDateTime;
use mizu_crypto::keys::{IdentityPublicKey, PrekeyPublicKey};
use mizu_crypto::x3dh::X3DHClient;
use mizu_crypto::Client;
use mizu_sqlite::MizuConnection;
use mizu_sqlite::{contact::Contact, identity::Identity, message::Message};
use mizu_tezos::TezosRpc;
use mizu_tezos_interface::{BoxedTezos, Tezos};
use rand::{CryptoRng, RngCore};
use std::convert::TryInto;
use std::fmt::{Debug, Display};
use std::path::PathBuf;
use std::rc::Rc;
use thiserror::Error;

pub mod contract;
pub mod faucet;

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
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Invalid message")]
    InvalidMessage(bincode::Error),
}

pub type DriverResult<T, A> =
    Result<A, DriverError<<T as Tezos>::ReadError, <T as Tezos>::WriteError>>;

// TODO: better name
struct ClientAndTimestamp {
    client: Client,
    latest_message_timestamp: Option<NaiveDateTime>,
}

/// Data associated with each user in Tezos
struct TezosData {
    identity_key: IdentityPublicKey,
    prekey: PrekeyPublicKey,
    postal_box: Vec<mizu_tezos_interface::Message>,
    pokes: Vec<Vec<u8>>,
}

// All states needed to run protocols are saved to a SQLite database and retrieved on demand.
pub struct Driver<T> {
    conn: Rc<MizuConnection>,
    tezos: T,
}

impl<T> Driver<T>
where
    T: Tezos,
{
    pub fn new(conn: Rc<MizuConnection>, tezos: T) -> Self {
        Self { conn, tezos }
    }

    pub fn boxed<'a>(self) -> Driver<BoxedTezos<'a>>
    where
        T: 'a,
    {
        Driver {
            conn: self.conn,
            tezos: self.tezos.boxed(),
        }
    }

    pub fn list_identities(&self) -> DriverResult<T, Vec<Identity>> {
        self.conn.list_identities().map_err(DriverError::UserData)
    }

    pub fn list_contacts(&self) -> DriverResult<T, Vec<Contact>> {
        self.conn.list_contacts().map_err(DriverError::UserData)
    }

    pub fn list_messages(
        &self,
        our_identity_id: i32,
        their_contact_id: i32,
    ) -> DriverResult<T, Vec<Message>> {
        self.conn
            .find_messages(our_identity_id, their_contact_id)
            .map_err(DriverError::UserData)
    }

    pub fn generate_identity<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        name: &str,
    ) -> DriverResult<T, ()> {
        let x3dh = X3DHClient::new(rng);
        self.conn
            .create_identity(name, self.tezos.address(), self.tezos.secret_key(), &x3dh)
            .map_err(DriverError::UserData)
    }

    /// publish local identity to Tezos
    pub fn publish_identity(&self, identity_id: i32) -> DriverResult<T, ()> {
        use DriverError::*;

        let identity = self.conn.find_identity(identity_id).map_err(UserData)?;
        let x3dh: X3DHClient = deserialize(&identity.x3dh_client).map_err(InvalidX3DH)?;
        self.tezos
            .register(
                Some(x3dh.identity_key.public_key.0.as_bytes()),
                x3dh.prekey.public_key.0.as_bytes(),
            )
            .map_err(TezosWrite)
    }

    pub fn add_contact(&self, name: &str, address: &str) -> DriverResult<T, ()> {
        self.conn
            .create_contact(name, address)
            .map_err(DriverError::UserData)
    }

    pub fn find_user(
        &self,
        address: &str,
    ) -> DriverResult<T, Option<mizu_tezos_interface::UserData>> {
        self.tezos
            .retrieve_user_data(address)
            .map_err(DriverError::TezosRead)
    }

    fn find_client(
        &self,
        our_identity_id: i32,
        their_contact_id: i32,
    ) -> DriverResult<T, Option<ClientAndTimestamp>> {
        use DriverError::*;

        self.conn
            .find_client(our_identity_id, their_contact_id)
            .map_err(UserData)?
            .map(|client| {
                Ok(ClientAndTimestamp {
                    client: deserialize(&client.client_data).map_err(InvalidClient)?,
                    latest_message_timestamp: client.latest_message_timestamp,
                })
            })
            .transpose()
    }

    fn find_or_create_client(
        &self,
        our_identity_id: i32,
        their_contact_id: i32,
        our_x3dh: &[u8],
        their_address: &str,
    ) -> DriverResult<T, ClientAndTimestamp> {
        Ok(self
            .find_client(our_identity_id, their_contact_id)?
            .unwrap_or_else(|| {
                // Construct a new Client from X3DHClient.

                // This unwrap() trusts the local SQLite database.
                let our_x3dh: X3DHClient = deserialize(our_x3dh).unwrap();
                ClientAndTimestamp {
                    client: Client::with_x3dh_client(
                        our_x3dh,
                        self.tezos.address().as_bytes(),
                        their_address.as_bytes(),
                    ),
                    latest_message_timestamp: None,
                }
            }))
    }

    fn retrieve_tezos_data(&self, address: &str) -> DriverResult<T, Option<TezosData>> {
        use DriverError::*;

        self.tezos
            .retrieve_user_data(address)
            .map_err(TezosRead)?
            .map(|data| {
                let identity_key: [u8; 32] = data
                    .identity_key
                    .as_slice()
                    .try_into()
                    .map_err(|_| InvalidKeyLength)?;
                let identity_key = IdentityPublicKey(identity_key.into());
                let prekey: [u8; 32] = data
                    .prekey
                    .as_slice()
                    .try_into()
                    .map_err(|_| InvalidKeyLength)?;
                let prekey = PrekeyPublicKey(prekey.into());

                Ok(TezosData {
                    identity_key,
                    prekey,
                    postal_box: data.postal_box,
                    pokes: data.pokes,
                })
            })
            .transpose()
    }

    // TODO: what if posting to Tezos succeeds but saving to SQLite fails?
    pub fn post_message<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        our_identity_id: i32,
        their_contact_id: i32,
        message: &str,
    ) -> DriverResult<T, ()> {
        use DriverError::*;

        let our_identity = self.conn.find_identity(our_identity_id).map_err(UserData)?;
        let their_contact = self.conn.find_contact(their_contact_id).map_err(UserData)?;

        match self.retrieve_tezos_data(&their_contact.address)? {
            Some(data) => {
                let ClientAndTimestamp {
                    mut client,
                    latest_message_timestamp,
                } = self.find_or_create_client(
                    our_identity_id,
                    their_contact_id,
                    &our_identity.x3dh_client,
                    &their_contact.address,
                )?;

                // Encrypt message and increment a ratchet.
                // TODO: I don't know this unwrap() may panic or not. Any thoughts? > mt_caret
                let message = client
                    .create_message(rng, &data.identity_key, &data.prekey, message.as_bytes())
                    .unwrap();

                // Post to Tezos.
                // This should be panic-free
                let payload = serialize(&message).unwrap();
                self.tezos.post(&[&payload], &[]).map_err(TezosWrite)?;

                // Save the sent message.
                self.conn
                    .create_message(our_identity_id, their_contact_id, &payload, true)
                    .map_err(UserData)?;

                // Save the incremented Client.
                self.conn
                    .upsert_client(
                        our_identity_id,
                        their_contact_id,
                        &client,
                        latest_message_timestamp.as_ref(),
                    )
                    .map_err(UserData)?;

                // wait a sec so that the next message will have distinct timestamp
                // TODO: better handling
                std::thread::sleep(std::time::Duration::from_secs(1));

                Ok(())
            }
            None => Err(NotFound),
        }
    }

    // TODO: what if retrieving from Tezos succeeds but saving to SQLite fails?
    pub fn get_messages<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        our_identity_id: i32,
        their_contact_id: i32,
    ) -> DriverResult<T, Vec<Vec<u8>>> {
        use DriverError::*;

        let our_identity = self.conn.find_identity(our_identity_id).map_err(UserData)?;
        let their_contact = self.conn.find_contact(their_contact_id).map_err(UserData)?;

        match self.retrieve_tezos_data(&their_contact.address)? {
            Some(data) => {
                let ClientAndTimestamp {
                    mut client,
                    mut latest_message_timestamp,
                } = self.find_or_create_client(
                    our_identity_id,
                    their_contact_id,
                    &our_identity.x3dh_client,
                    &their_contact.address,
                )?;

                let mut messages = vec![];
                for message in data.postal_box.iter() {
                    // assuming messages are ordered from older to newer
                    match latest_message_timestamp {
                        // if the recorded timestamp is newer than message's timestamp, skip it.
                        Some(latest_message_timestamp)
                            if latest_message_timestamp >= message.timestamp =>
                        {
                            continue;
                        }
                        // otherwise, update the timestamp.
                        _ => {
                            latest_message_timestamp = Some(message.timestamp);
                        }
                    }

                    let message = deserialize(&message.content).map_err(InvalidMessage)?;
                    if let Ok(message) = client.attempt_message_decryption(rng, message) {
                        self.conn
                            .create_message(our_identity_id, their_contact_id, &message, false)
                            .map_err(UserData)?;
                        messages.push(message);
                    }
                }

                self.conn
                    .upsert_client(
                        our_identity_id,
                        their_contact_id,
                        &client,
                        latest_message_timestamp.as_ref(),
                    )
                    .map_err(UserData)?;

                Ok(messages)
            }
            None => Err(NotFound),
        }
    }

    pub fn get_pokes(&self) -> DriverResult<T, Vec<Vec<u8>>> {
        use DriverError::*;

        match self.retrieve_tezos_data(self.tezos.address())? {
            Some(data) => Ok(data.pokes),
            None => Err(NotFound),
        }
    }
}

pub fn create_rpc_driver(
    faucet_output: &PathBuf,
    contract_config: &PathBuf,
    db_path: &str,
) -> Result<Driver<TezosRpc>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    use std::fs::read_to_string;

    // Surprisingly, reading the whole file is faster.
    // https://github.com/serde-rs/json/issues/160#issuecomment-253446892
    let faucet_output: faucet::FaucetOutput =
        serde_json::from_str(&read_to_string(faucet_output)?)?;
    let contract_config: contract::ContractConfig =
        serde_json::from_str(&read_to_string(contract_config)?)?;
    let host = contract_config.rpc_host.parse()?;
    let tezos = TezosRpc::new(
        contract_config.debug,
        host,
        faucet_output.pkh,
        faucet_output.secret,
        contract_config.contract_address,
    );
    let conn = Rc::new(MizuConnection::connect(db_path)?);

    Ok(Driver::new(conn, tezos))
}

// ensure test related code (especially migration SQL) is not included in the binary
#[cfg(test)]
mod test {
    use super::*;
    use diesel::{connection::SimpleConnection, prelude::*};
    use mizu_sqlite::MizuConnection;
    use mizu_tezos_mock::TezosMock;
    use rand::rngs::OsRng;
    use std::rc::Rc;

    fn prepare_user_database() -> Rc<MizuConnection> {
        // Create an in-memory SQLite database
        let conn = SqliteConnection::establish(":memory:").unwrap();
        let migration =
            include_str!("../../mizu-sqlite/migrations/2020-06-16-100417_initial/up.sql");
        conn.batch_execute(migration).unwrap();

        Rc::new(MizuConnection::new(conn))
    }

    #[test]
    fn test_smoke_1() {
        // use Tezos address
        let alice_address = "alice".to_string();
        let alice_secret_key = "alice".to_string();
        let bob_address = "bob".to_string();
        let bob_secret_key = "bob".to_string();

        let mock_conn = Rc::new({
            // Create an in-memory SQLite database
            let conn = SqliteConnection::establish(":memory:").unwrap();
            let migration =
                include_str!("../../mizu-tezos-mock/migrations/2020-06-17-013029_initial/up.sql");
            conn.batch_execute(migration).unwrap();
            conn
        });

        let mut rng = OsRng;

        let alice = {
            let user_database = prepare_user_database();
            let tezos_mock = TezosMock::new(
                alice_address.clone(),
                alice_secret_key,
                Rc::clone(&mock_conn),
            );
            Driver::new(user_database, tezos_mock)
        };
        let bob = {
            let user_database = prepare_user_database();
            // use Tezos address
            let tezos_mock = TezosMock::new(bob_address.clone(), bob_secret_key, mock_conn);
            Driver::new(user_database, tezos_mock)
        };

        // first, each user generates identity and uploads to Tezos.
        alice
            .generate_identity(&mut rng, "alice's identity")
            .unwrap();
        alice.publish_identity(1).unwrap();
        bob.generate_identity(&mut rng, "bob's identity").unwrap();
        bob.publish_identity(1).unwrap();

        // next, each user adds each other to the contact list (poke is not implemented yet)
        alice.add_contact("bob's address", &bob_address).unwrap();
        bob.add_contact("alice's address", &alice_address).unwrap();

        // alice sends some messages to bob.
        alice
            .post_message(&mut rng, 1, 1, "Hello from alice!")
            .unwrap();
        alice
            .post_message(&mut rng, 1, 1, "waiting for response...")
            .unwrap();

        // bob receives the messages
        let messages = bob.get_messages(&mut rng, 1, 1).unwrap();
        assert_eq!(
            messages,
            [b"Hello from alice!" as &[u8], b"waiting for response...",]
        );

        // bob replies
        bob.post_message(&mut rng, 1, 1, "こんにちは").unwrap();

        // alice receives the reply
        let messages = alice.get_messages(&mut rng, 1, 1).unwrap();
        assert_eq!(messages, ["こんにちは".as_bytes(),]);
    }
}
