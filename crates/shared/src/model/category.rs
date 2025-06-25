use sqlx::prelude::FromRow;


#[derive(Debug, FromRow, Clone)]
pub struct Category {
    pub id: i32,
    pub name: String,
}
