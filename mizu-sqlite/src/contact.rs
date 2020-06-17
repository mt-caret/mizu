use crate::schema::*;

#[derive(Queryable)]
pub struct Contact {
    pub id: i32,
    pub public_key: Vec<u8>,
    pub name: String,
    pub created_at: String,
}

#[derive(Insertable)]
#[table_name = "contacts"]
pub struct NewContact<'a> {
    pub public_key: &'a [u8],
    pub name: &'a str,
}