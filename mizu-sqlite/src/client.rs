use crate::schema::*;

#[derive(Queryable)]
pub struct Client {
    pub identity_id: i32,
    pub contact_id: i32,
    pub client_data: Vec<u8>,
}

#[derive(Insertable)]
#[table_name = "clients"]
pub struct NewClient<'a> {
    pub identity_id: i32,
    pub contact_id: i32,
    pub client_data: &'a [u8],
}
