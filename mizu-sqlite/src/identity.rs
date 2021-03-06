use crate::schema::*;

#[derive(Debug, Queryable)]
pub struct Identity {
    pub id: i32,
    pub name: String,
    pub address: String,
    pub secret_key: String,
    pub x3dh_client: Vec<u8>,
    pub created_at: String,
}

#[derive(Insertable)]
#[table_name = "identities"]
pub struct NewIdentity<'a> {
    pub name: &'a str,
    pub address: &'a str,
    pub secret_key: &'a str,
    pub x3dh_client: &'a [u8],
}
