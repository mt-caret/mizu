use crate::schema::*;

#[derive(Queryable)]
pub struct Identity {
    pub id: i32,
    pub name: String,
    pub address: String,
    pub x3dh_client: Vec<u8>,
    pub created_at: String,
}

#[derive(Insertable)]
#[table_name = "identities"]
pub struct NewIdentity<'a> {
    pub name: &'a str,
    pub address: &'a str,
    pub x3dh_client: &'a [u8],
}
