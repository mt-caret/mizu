use crate::schema::*;
use chrono::naive::NaiveDateTime;

#[derive(Queryable)]
pub struct Client {
    pub identity_id: i32,
    pub contact_id: i32,
    pub client_data: Vec<u8>,
    pub latest_message_timestamp: Option<NaiveDateTime>,
}

#[derive(Queryable)]
pub struct ClientWithName {
    pub contact_id: i32,
    pub address: String,
    pub name: String,
    pub latest_message_timestamp: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name = "clients"]
pub struct NewClient<'a> {
    pub identity_id: i32,
    pub contact_id: i32,
    pub client_data: &'a [u8],
    pub latest_message_timestamp: Option<&'a NaiveDateTime>,
}

#[derive(AsChangeset)]
#[table_name = "clients"]
pub struct UpdateClient<'a> {
    pub client_data: &'a [u8],
    pub latest_message_timestamp: Option<&'a NaiveDateTime>,
}
