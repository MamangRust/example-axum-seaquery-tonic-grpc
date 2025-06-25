use sqlx::prelude::FromRow;



#[derive(Debug, FromRow, Clone)]
pub struct Comment {
    pub id: i32,
    pub id_post_comment: i32,
    pub user_name_comment: String,
    pub comment: String,
}
