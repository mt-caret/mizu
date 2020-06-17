use crate::schema::users;

#[derive(Identifiable, Queryable)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub address: Vec<u8>,
    pub identity_key: Vec<u8>,
    pub prekey: Vec<u8>,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub address: &'a [u8],
    pub identity_key: &'a [u8],
    pub prekey: &'a [u8],
}
