#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use mizu_crypto::x3dh::X3DHClient;
use mizu_crypto::Client;

pub mod client;
pub mod contact;
pub mod identity;
pub mod message;

mod schema;

pub struct MizuConnection {
    conn: SqliteConnection,
}

impl MizuConnection {
    pub fn create_identity(&self, name: &str, x3dh: &X3DHClient) {
        diesel::insert_into(schema::identities::table)
            .values(&identity::NewIdentity {
                name,
                x3dh_client: &bincode::serialize(&x3dh).unwrap(),
            })
            .execute(&self.conn)
            .unwrap();
    }

    pub fn list_identities(&self) -> Vec<identity::Identity> {
        schema::identities::dsl::identities
            .load::<identity::Identity>(&self.conn)
            .unwrap()
    }

    pub fn find_identities(&self, needle: &str) -> Vec<identity::Identity> {
        use schema::identities::dsl::*;

        identities
            .filter(name.eq(needle))
            .load::<identity::Identity>(&self.conn)
            .unwrap()
    }

    pub fn update_identity(&self, id: i32, name: &str, x3dh: &X3DHClient) {
        use schema::identities::dsl;

        let target = dsl::identities.find(id);
        diesel::update(target)
            .set((
                dsl::name.eq(name),
                dsl::x3dh_client.eq(bincode::serialize(&x3dh).unwrap()),
            ))
            .execute(&self.conn)
            .unwrap();
    }

    pub fn create_contact(&self, name: &str, public_key: &[u8]) {
        diesel::insert_into(schema::contacts::table)
            .values(&contact::NewContact { name, public_key })
            .execute(&self.conn)
            .unwrap();
    }

    pub fn list_contacts(&self) -> Vec<contact::Contact> {
        schema::contacts::dsl::contacts
            .load::<contact::Contact>(&self.conn)
            .unwrap()
    }

    pub fn find_contacts(&self, needle: &str) -> Vec<contact::Contact> {
        use schema::contacts::dsl::*;

        contacts
            .filter(name.eq(needle))
            .load::<contact::Contact>(&self.conn)
            .unwrap()
    }

    pub fn create_client(&self, identity_id: i32, contact_id: i32, client_data: &[u8]) {
        diesel::insert_into(schema::clients::table)
            .values(&client::NewClient {
                identity_id,
                contact_id,
                client_data,
            })
            .execute(&self.conn)
            .unwrap();
    }

    pub fn list_clients(&self) -> Vec<client::Client> {
        schema::clients::dsl::clients
            .load::<client::Client>(&self.conn)
            .unwrap()
    }

    pub fn find_client(&self, identity_id: i32, contact_id: i32) -> client::Client {
        use schema::clients::dsl;

        dsl::clients
            .find((identity_id, contact_id))
            .first(&self.conn)
            .unwrap()
    }

    pub fn update_client(&self, identity_id: i32, contact_id: i32, client: &Client) {
        use schema::clients::dsl;

        let target = dsl::clients.find((identity_id, contact_id));
        diesel::update(target)
            .set(dsl::client_data.eq(bincode::serialize(client).unwrap()))
            .execute(&self.conn)
            .unwrap();
    }

    pub fn create_message(&self, identity_id: i32, contact_id: i32, content: &[u8]) {
        diesel::insert_into(schema::messages::table)
            .values(&message::NewMessage {
                identity_id,
                contact_id,
                content,
            })
            .execute(&self.conn)
            .unwrap();
    }

    pub fn find_messages(&self, identity_id: i32, contact_id: i32) -> Vec<message::Message> {
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
            .unwrap()
    }
}

pub fn connect(url: &str) -> Result<MizuConnection, ConnectionError> {
    Ok(MizuConnection {
        conn: SqliteConnection::establish(url)?,
    })
}
