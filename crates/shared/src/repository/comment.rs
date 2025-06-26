use async_trait::async_trait;
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;

use crate::abstract_trait::CommentRepositoryTrait;
use crate::config::ConnectionPool;
use crate::domain::{CreateCommentRequest, UpdateCommentRequest};
use crate::model::comment::Comment;
use crate::schema::comment::Comments;
use crate::utils::AppError;

pub struct CommentRepository {
    db_pool: ConnectionPool,
}

impl CommentRepository {
    pub fn new(db_pool: ConnectionPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl CommentRepositoryTrait for CommentRepository {
    async fn find_all(&self) -> Result<Vec<Comment>, AppError> {
        let query = Query::select()
            .columns([
                Comments::Id,
                Comments::IdPostComment,
                Comments::UserNameComment,
                Comments::Comment,
            ])
            .from(Comments::Table)
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = query;

        let results = sqlx::query_as_with::<_, Comment, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await?;

        Ok(results)
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Comment>, AppError> {
        let query = Query::select()
            .columns([
                Comments::Id,
                Comments::IdPostComment,
                Comments::UserNameComment,
                Comments::Comment,
            ])
            .from(Comments::Table)
            .and_where(Expr::col(Comments::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = query;

        let result = sqlx::query_as_with::<_, Comment, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(result)
    }

    async fn create(&self, input: &CreateCommentRequest) -> Result<Comment, AppError> {
        let insert = Query::insert()
            .into_table(Comments::Table)
            .columns([
                Comments::IdPostComment,
                Comments::UserNameComment,
                Comments::Comment,
            ])
            .values_panic([
                input.id_post_comment.into(),
                input.user_name_comment.clone().into(),
                input.comment.clone().into(),
            ])
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = insert;

        let row = sqlx::query_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await?;
        let id: i32 = sqlx::Row::try_get(&row, 0)?;

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::SqlxError(sqlx::Error::RowNotFound))
    }

    async fn update(&self, input: &UpdateCommentRequest) -> Result<Comment, AppError> {
        let id = input
            .id_post_comment
            .ok_or_else(|| AppError::ValidationError("ID is required".into()))?;

        let update = Query::update()
            .table(Comments::Table)
            .values(vec![
                (
                    Comments::UserNameComment,
                    input.user_name_comment.clone().into(),
                ),
                (Comments::Comment, input.comment.clone().into()),
            ])
            .and_where(Expr::col(Comments::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = update;

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::SqlxError(sqlx::Error::RowNotFound));
        }

        self.find_by_id(id)
            .await?
            .ok_or_else(|| AppError::SqlxError(sqlx::Error::RowNotFound))
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        let delete = Query::delete()
            .from_table(Comments::Table)
            .and_where(Expr::col(Comments::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = delete;

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::SqlxError(sqlx::Error::RowNotFound));
        }

        Ok(())
    }
}
