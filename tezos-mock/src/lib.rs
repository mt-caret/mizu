//! TODO: error handling
//! TODO: implementation

#[macro_use]
extern crate diesel;

use diesel::prelude::*;
use tezos_interface::*;

pub struct TezosMock {
    conn: SqliteConnection,
}

impl TezosMock {
    pub fn new(url: &str) -> Self {
        TezosMock {
            conn: SqliteConnection::establish(url).unwrap(),
        }
    }
}

impl Tezos for TezosMock {
    fn retrieve_user_data(&self, address: &[u8]) -> UserData {
        todo!()
    }

    fn post(&self, sender_address: &[u8], add: &[&[u8]], remove: &[&usize]) {
        todo!()
    }

    fn poke(&self, target_address: &[u8], data: &[u8]) {
        todo!()
    }

    fn register(&self, sender_address: &[u8], identity_key: &[u8], prekey: &[u8]) {
        todo!()
    }
}
