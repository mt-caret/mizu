//! TODO: error handling

#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use mizu_tezos_interface::*;
use std::rc::Rc;

mod message;
mod poke;
mod schema;
mod user;

macro_rules! dbg_query {
    ($query:expr) => {{
        eprintln!(
            "{}",
            diesel::debug_query::<diesel::sqlite::Sqlite, _>(&$query)
        );
        $query
    }};
}

type DieselError = diesel::result::Error;

pub struct TezosMock {
    /// Tezos address
    address: String,
    secret_key: String,
    conn: Rc<SqliteConnection>,
}

impl TezosMock {
    pub fn new(address: String, secret_key: String, conn: Rc<SqliteConnection>) -> Self {
        TezosMock {
            address: address,
            secret_key: secret_key,
            conn,
        }
    }

    /*
    pub fn connect(address: &'a str, url: &str) -> ConnectionResult<Self> {
        Ok(TezosMock {
            address,
            conn: SqliteConnection::establish(url)?,
        })
    }
    */
}

impl Tezos for TezosMock {
    type ReadError = DieselError;
    type WriteError = DieselError;

    fn address(&self) -> &str {
        &self.address
    }

    fn secret_key(&self) -> &str {
        &self.secret_key
    }

    fn retrieve_user_data(&self, address: &str) -> Result<Option<UserData>, Self::ReadError> {
        // According to https://docs.diesel.rs/diesel/associations/index.html,
        // selecting three tables is better than joining them.
        // TODO: run queries within a transaction?
        use schema::messages::dsl as messages_dsl;
        use schema::pokes::dsl as pokes_dsl;
        use schema::users::dsl as users_dsl;

        if let Some(user) = users_dsl::users
            .filter(users_dsl::address.eq(address))
            .first::<user::User>(&*self.conn)
            .optional()?
        {
            let messages = message::Message::belonging_to(&user)
                .order(messages_dsl::id.asc())
                .load::<message::Message>(&*self.conn)?;
            let pokes = poke::Poke::belonging_to(&user)
                .order(pokes_dsl::id.asc())
                .load::<poke::Poke>(&*self.conn)?;

            Ok(Some(UserData {
                identity_key: user.identity_key,
                prekey: user.prekey,
                postal_box: messages
                    .into_iter()
                    .map(|m| Message {
                        content: m.content,
                        timestamp: m.timestamp,
                    })
                    .collect(),
                pokes: pokes.into_iter().map(|p| p.content).collect(),
            }))
        } else {
            Ok(None)
        }
    }

    fn post(&self, add: &[&[u8]], remove: &[&usize]) -> Result<(), Self::WriteError> {
        use schema::messages::dsl as messages_dsl;
        use schema::users::dsl as users_dsl;

        // TODO: transaction?
        // First, retrieve all our posts to determine ones to be removed.
        let user = users_dsl::users
            .filter(users_dsl::address.eq(&self.address))
            .first::<user::User>(&*self.conn)?;
        let messages = message::Message::belonging_to(&user)
            .order(messages_dsl::id.asc())
            .load::<message::Message>(&*self.conn)?;
        // TODO: return an error if the index is out of bounds (panics now).
        let remove: Vec<i32> = remove.iter().map(|i| messages[**i].id).collect();

        // Next, remove the corresponding messages.
        diesel::delete(messages_dsl::messages.filter(messages_dsl::id.eq_any(&remove)))
            .execute(&*self.conn)?;

        // Finally, add messages.
        let new_messages: Vec<_> = add
            .iter()
            .map(|content| message::NewMessage {
                user_id: user.id,
                content,
            })
            .collect();

        for new_message in new_messages.iter() {
            let _ = dbg_query!(diesel::insert_into(schema::messages::table).values(new_message));
        }
        diesel::insert_into(schema::messages::table)
            .values(&new_messages)
            .execute(&*self.conn)?;

        Ok(())
    }

    fn poke(&self, target_address: &str, data: &[u8]) -> Result<(), Self::WriteError> {
        // TODO: transaction?
        use schema::users::dsl;
        let user_id = dsl::users
            .filter(dsl::address.eq(target_address))
            .select(dsl::id)
            .first::<i32>(&*self.conn)?;

        diesel::insert_into(schema::pokes::table)
            .values(&poke::NewPoke {
                user_id,
                content: data,
            })
            .execute(&*self.conn)?;

        Ok(())
    }

    fn register(&self, identity_key: Option<&[u8]>, prekey: &[u8]) -> Result<(), Self::WriteError> {
        use schema::users::dsl;

        match identity_key {
            // CR pandaman: Is it okay to fail silently if no matching row exist?
            // We can check if the number of affected rows equals to zero or one.
            None => dbg_query!(
                diesel::update(dsl::users.filter(dsl::address.eq(&self.address)))
                    .set(dsl::prekey.eq(prekey))
            )
            .execute(&*self.conn)?,
            Some(identity_key) => {
                // As our schema declares address column to be unique, this query
                // - updates identity_key and prekey if the address already exists; or
                // - inserts a new row with the given keys if the address does not exist.
                dbg_query!(diesel::replace_into(dsl::users).values(&user::NewUser {
                    address: &self.address,
                    identity_key,
                    prekey,
                }))
                .execute(&*self.conn)?
            }
        };

        Ok(())
    }
}
