use anyhow::{Context, Result};
use async_trait::async_trait;
use sea_query::{Expr, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use tracing::{error, info};

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
            .await
            .context("Failed to fetch comments")?;

        Ok(results)
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Comment>, AppError> {
        info!("Finding comment by id: {}", id);

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
            .await
            .map_err(AppError::from)?;

        info!("Find result: {:?}", result);

        Ok(result)
    }

    async fn create(&self, input: &CreateCommentRequest) -> Result<Comment, AppError> {
        info!("Creating new comment");

        let query = Query::insert()
            .into_table(Comments::Table)
            .columns([
                Comments::IdPostComment,
                Comments::UserNameComment,
                Comments::Comment,
            ])
            .values([
                input.id_post_comment.into(),
                input.user_name_comment.clone().into(),
                input.comment.clone().into(),
            ])
            .unwrap()
            .to_owned()
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = query;

        let result = sqlx::query_as_with::<_, Comment, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::from)?;

        info!("New comment inserted with ID: {}", result.id);

        Ok(result)
    }

    async fn update(&self, input: &UpdateCommentRequest) -> Result<Comment, AppError> {
        info!(
            "Updating comment ID {} with new name '{}'",
            input.id_post_comment, input.user_name_comment
        );

        let (sql, values) = Query::update()
            .table(Comments::Table)
            .values(vec![
                (
                    Comments::UserNameComment,
                    input.user_name_comment.clone().into(),
                ),
                (Comments::Comment, input.comment.clone().into()),
            ])
            .and_where(Expr::col(Comments::Id).eq(input.id_post_comment))
            .build_sqlx(PostgresQueryBuilder);

        let affected = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await?
            .rows_affected();

        if affected == 0 {
            error!("Comment ID {} not found for update", input.id_post_comment);
            return Err(AppError::NotFound(format!(
                "Comment with ID {} not found",
                input.id_post_comment
            )));
        }

        let comment = self
            .find_by_id(input.id_post_comment)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "Comment with ID {} not found",
                    input.id_post_comment
                ))
            })?;

        info!("Successfully updated comment ID {}", input.id_post_comment);
        Ok(comment)
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        let (sql, values) = Query::delete()
            .from_table(Comments::Table)
            .and_where(Expr::col(Comments::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await?;

        match result.rows_affected() {
            0 => {
                error!("No comment found to delete with ID: {id}");
                Err(AppError::NotFound(format!(
                    "Comment with ID {id} not found"
                )))
            }
            _ => {
                info!("Comment ID: {id} deleted successfully");
                Ok(())
            }
        }
    }
}
