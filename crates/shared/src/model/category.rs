use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: i32,
    pub name: String,
}
