use chrono::NaiveDateTime;

use crate::schema::messages;
use crate::user::User;

#[derive(Identifiable, Queryable, Associations)]
#[belongs_to(User)]
#[table_name = "messages"]
pub struct Message {
    pub id: i32,
    pub user_id: i32,
    pub content: Vec<u8>,
    pub timestamp: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "messages"]
pub struct NewMessage<'a> {
    pub user_id: i32,
    pub content: &'a [u8],
}
