use crate::schema::pokes;
use crate::user::User;

#[derive(Identifiable, Queryable, Associations)]
#[belongs_to(User)]
#[table_name = "pokes"]
pub struct Poke {
    pub id: i32,
    pub user_id: i32,
    pub content: Vec<u8>,
}

#[derive(Insertable)]
#[table_name = "pokes"]
pub struct NewPoke<'a> {
    pub user_id: i32,
    pub content: &'a [u8],
}
