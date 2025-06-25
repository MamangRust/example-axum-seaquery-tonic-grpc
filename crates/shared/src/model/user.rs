use sqlx::prelude::FromRow;

#[derive(Debug, FromRow, Clone)]
pub struct User {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub password: String,
}
