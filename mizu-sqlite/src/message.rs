use crate::schema::*;
use chrono::naive::NaiveDateTime;

#[derive(Debug, Queryable)]
pub struct Message {
    pub id: i32,
    pub identity_id: i32,
    pub contact_id: i32,
    pub content: Vec<u8>,
    pub my_message: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "messages"]
pub struct NewMessage<'a> {
    pub identity_id: i32,
    pub contact_id: i32,
    pub content: &'a [u8],
    pub my_message: bool,
    pub created_at: NaiveDateTime,
}
