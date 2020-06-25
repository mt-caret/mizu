use crate::schema::*;

#[derive(Debug, Queryable)]
pub struct Contact {
    pub id: i32,
    pub address: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Insertable)]
#[table_name = "contacts"]
pub struct NewContact<'a> {
    pub address: &'a str,
    pub name: &'a str,
}
