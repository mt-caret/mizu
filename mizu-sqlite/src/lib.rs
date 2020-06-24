// All of the `unwrap()`s in this module are serializing clients and they should succeed.

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate diesel_migrations;

use chrono::naive::NaiveDateTime;
use diesel::prelude::*;
use diesel_migrations::embed_migrations;
use mizu_crypto::x3dh::X3DHClient;
use mizu_crypto::Client;

pub mod client;
pub mod contact;
pub mod identity;
pub mod message;

mod schema;

type Result<T> = std::result::Result<T, diesel::result::Error>;

pub struct MizuConnection {
    conn: SqliteConnection,
}

embed_migrations!();

impl MizuConnection {
    pub fn new(conn: SqliteConnection) -> Self {
        MizuConnection { conn }
    }

    pub fn connect(url: &str) -> std::result::Result<Self, ConnectionError> {
        let run_migration = url == ":memory:" || std::fs::metadata(url).is_err();

        let mizu_connection = Self {
            conn: SqliteConnection::establish(url)?,
        };

        if run_migration {
            mizu_connection.run_migrations();
        }

        Ok(mizu_connection)
    }

    // TODO: should probably check for errors
    // TODO: embedded_migrations::run_with_output will redirect output instead
    // of throwing it away, should log this.
    pub fn run_migrations(&self) {
        embedded_migrations::run(&self.conn).expect("migration should never fail");
    }

    pub fn create_identity(
        &self,
        name: &str,
        address: &str,
        secret_key: &str,
        x3dh: &X3DHClient,
    ) -> Result<()> {
        diesel::insert_into(schema::identities::table)
            .values(&identity::NewIdentity {
                name,
                address,
                secret_key,
                x3dh_client: &bincode::serialize(&x3dh).unwrap(),
            })
            .execute(&self.conn)?;

        Ok(())
    }

    pub fn list_identities(&self) -> Result<Vec<identity::Identity>> {
        schema::identities::dsl::identities.load::<identity::Identity>(&self.conn)
    }

    pub fn find_identity(&self, id: i32) -> Result<identity::Identity> {
        use schema::identities::dsl::identities;

        identities.find(id).first::<identity::Identity>(&self.conn)
    }

    pub fn find_identity_by_name(&self, needle: &str) -> Result<identity::Identity> {
        use schema::identities::dsl::*;

        identities
            .filter(name.eq(needle))
            .first::<identity::Identity>(&self.conn)
    }

    pub fn update_identity(&self, id: i32, name: &str, x3dh: &X3DHClient) -> Result<()> {
        use schema::identities::dsl;

        let target = dsl::identities.find(id);
        diesel::update(target)
            .set((
                dsl::name.eq(name),
                dsl::x3dh_client.eq(bincode::serialize(&x3dh).unwrap()),
            ))
            .execute(&self.conn)?;

        Ok(())
    }

    pub fn create_contact(&self, name: &str, address: &str) -> Result<()> {
        diesel::insert_into(schema::contacts::table)
            .values(&contact::NewContact { name, address })
            .execute(&self.conn)?;

        Ok(())
    }

    pub fn list_contacts(&self) -> Result<Vec<contact::Contact>> {
        schema::contacts::dsl::contacts.load::<contact::Contact>(&self.conn)
    }

    pub fn find_contact(&self, contact_id: i32) -> Result<contact::Contact> {
        use schema::contacts::dsl::contacts;

        contacts
            .find(contact_id)
            .first::<contact::Contact>(&self.conn)
    }

    pub fn find_contacts(&self, needle: &str) -> Result<Vec<contact::Contact>> {
        use schema::contacts::dsl::*;

        contacts
            .filter(name.eq(needle))
            .load::<contact::Contact>(&self.conn)
    }

    pub fn create_client(
        &self,
        identity_id: i32,
        contact_id: i32,
        client: &Client,
        latest_message_timestamp: Option<&NaiveDateTime>,
    ) -> Result<()> {
        diesel::insert_into(schema::clients::table)
            .values(&client::NewClient {
                identity_id,
                contact_id,
                client_data: &bincode::serialize(client).unwrap(),
                latest_message_timestamp,
            })
            .execute(&self.conn)?;

        Ok(())
    }

    pub fn list_clients(&self) -> Result<Vec<client::Client>> {
        schema::clients::dsl::clients.load::<client::Client>(&self.conn)
    }

    pub fn list_talking_clients(&self, identity_id: i32) -> Result<Vec<client::ClientInfo>> {
        use schema::clients::dsl as clients_dsl;
        use schema::contacts::dsl as contacts_dsl;

        schema::clients::table
            .inner_join(schema::contacts::table)
            .filter(clients_dsl::identity_id.eq(identity_id))
            .select((
                contacts_dsl::id,
                contacts_dsl::address,
                contacts_dsl::name,
                clients_dsl::latest_message_timestamp,
            ))
            .load::<client::ClientInfo>(&self.conn)
    }

    pub fn find_client(&self, identity_id: i32, contact_id: i32) -> Result<Option<client::Client>> {
        use schema::clients::dsl;

        dsl::clients
            .find((identity_id, contact_id))
            .first(&self.conn)
            .optional()
    }

    pub fn update_client(
        &self,
        identity_id: i32,
        contact_id: i32,
        client: &Client,
        latest_message_timestamp: Option<&NaiveDateTime>,
    ) -> Result<()> {
        use schema::clients::dsl;

        let target = dsl::clients.find((identity_id, contact_id));
        diesel::update(target)
            .set(client::UpdateClient {
                client_data: &bincode::serialize(client).unwrap(),
                latest_message_timestamp,
            })
            .execute(&self.conn)?;

        Ok(())
    }

    pub fn upsert_client(
        &self,
        identity_id: i32,
        contact_id: i32,
        client: &Client,
        latest_message_timestamp: Option<&NaiveDateTime>,
    ) -> Result<()> {
        diesel::replace_into(schema::clients::table)
            .values(&client::NewClient {
                identity_id,
                contact_id,
                client_data: &bincode::serialize(client).unwrap(),
                latest_message_timestamp,
            })
            .execute(&self.conn)?;

        Ok(())
    }

    pub fn create_message(
        &self,
        identity_id: i32,
        contact_id: i32,
        content: &[u8],
        my_message: bool,
    ) -> Result<()> {
        diesel::insert_into(schema::messages::table)
            .values(&message::NewMessage {
                identity_id,
                contact_id,
                content,
                my_message,
            })
            .execute(&self.conn)?;

        Ok(())
    }

    pub fn find_messages(
        &self,
        identity_id: i32,
        contact_id: i32,
    ) -> Result<Vec<message::Message>> {
        use schema::messages::dsl;

        // TODO: limit clause
        dsl::messages
            .filter(
                dsl::identity_id
                    .eq(identity_id)
                    .and(dsl::contact_id.eq(contact_id)),
            )
            .order_by(dsl::id.asc()) // assuming messages in the DB are not shuffled
            .load::<message::Message>(&self.conn)
    }
}
