use crate::abstract_trait::PostsRepositoryTrait;
use crate::config::ConnectionPool;
use crate::domain::{CreatePostRequest, PostRelationResponse, UpdatePostRequest};
use crate::utils::AppError;

use crate::model::posts::{Post, PostRelationModel};
use crate::schema::comment::Comments;
use crate::schema::posts::Posts;

use async_trait::async_trait;
use sea_query::{Expr, Func, JoinType, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use tracing::info;

pub struct PostRepository {
    db_pool: ConnectionPool,
}

impl PostRepository {
    pub fn new(db_pool: ConnectionPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl PostsRepositoryTrait for PostRepository {
    async fn get_all_posts(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Post>, i64), AppError> {
        let offset = (page - 1) * page_size;

        let mut select_query = Query::select();
        select_query
            .columns([
                (Posts::Table, Posts::Id),
                (Posts::Table, Posts::Title),
                (Posts::Table, Posts::Img),
                (Posts::Table, Posts::Body),
                (Posts::Table, Posts::CategoryId),
                (Posts::Table, Posts::UserId),
                (Posts::Table, Posts::UserName),
            ])
            .from(Posts::Table)
            .offset(offset as u64)
            .limit(page_size as u64);

        if let Some(ref s) = search {
            select_query
                .and_where(Expr::col((Posts::Table, Posts::Title)).like(format!("%{}%", s)));
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);

        let posts = sqlx::query_as_with::<_, Post, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await?;

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(Posts::Id)))
            .from(Posts::Table);

        if let Some(ref s) = search {
            count_query.and_where(Expr::col((Posts::Table, Posts::Title)).like(format!("%{}%", s)));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);
        let total: (i64,) = sqlx::query_as_with(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await?;

        Ok((posts, total.0))
    }

    async fn get_post(&self, post_id: i32) -> Result<Option<Post>, AppError> {
        let query = Query::select()
            .columns([
                Posts::Id,
                Posts::Title,
                Posts::Img,
                Posts::Body,
                Posts::CategoryId,
                Posts::UserId,
                Posts::UserName,
            ])
            .from(Posts::Table)
            .and_where(Expr::col(Posts::Id).eq(post_id))
            .to_owned();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_as_with::<_, Post, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        Ok(result)
    }

    async fn get_post_relation(&self, post_id: i32) -> Result<Vec<PostRelationResponse>, AppError> {
        let query = Query::select()
            .column((Posts::Table, Posts::Id))
            .column((Posts::Table, Posts::Title))
            .column((Comments::Table, Comments::Id))
            .column((Comments::Table, Comments::IdPostComment))
            .column((Comments::Table, Comments::UserNameComment))
            .column((Comments::Table, Comments::Comment))
            .from(Posts::Table)
            .join(
                JoinType::InnerJoin,
                Comments::Table,
                Expr::col((Posts::Table, Posts::Id))
                    .equals((Comments::Table, Comments::IdPostComment)),
            )
            .and_where(Expr::col((Posts::Table, Posts::Id)).eq(post_id))
            .to_owned();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

        let result: Vec<PostRelationModel> = sqlx::query_as_with(&sql, values)
            .fetch_all(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        let responses = result.into_iter().map(PostRelationResponse::from).collect();

        Ok(responses)
    }

    async fn create_post(&self, input: &CreatePostRequest) -> Result<Post, AppError> {
        let query = Query::insert()
            .into_table(Posts::Table)
            .columns([
                Posts::Title,
                Posts::Img,
                Posts::Body,
                Posts::CategoryId,
                Posts::UserId,
                Posts::UserName,
            ])
            .values_panic([
                input.title.clone().into(),
                input.file.clone().into(),
                input.body.clone().into(),
                input.category_id.into(),
                input.user_id.into(),
                input.user_name.clone().into(),
            ])
            .returning_all()
            .to_owned();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

        let post: Post = sqlx::query_as_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        Ok(post)
    }

    async fn update_post(&self, input: &UpdatePostRequest) -> Result<Post, AppError> {
        let id = input
            .post_id
            .ok_or_else(|| AppError::ValidationError("Post ID is required".into()))?;

        let query = Query::update()
            .table(Posts::Table)
            .values([
                (Posts::Title, input.title.clone().into()),
                (Posts::Body, input.body.clone().into()),
                (Posts::Img, input.file.clone().into()),
                (Posts::CategoryId, input.category_id.into()),
                (Posts::UserId, input.user_id.into()),
                (Posts::UserName, input.user_name.clone().into()),
            ])
            .and_where(Expr::col(Posts::Id).eq(id))
            .returning_all()
            .to_owned();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

        let post: Post = sqlx::query_as_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        Ok(post)
    }
    async fn delete_post(&self, post_id: i32) -> Result<(), AppError> {
        let query = Query::delete()
            .from_table(Posts::Table)
            .and_where(Expr::col(Posts::Id).eq(post_id))
            .to_owned();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await?;

        if result.rows_affected() == 0 {
            info!("No posts found to delete with ID: {}", post_id);
            return Err(AppError::SqlxError(sqlx::Error::RowNotFound));
        }

        info!("posts ID: {} deleted successfully", post_id);
        Ok(())
    }
}
