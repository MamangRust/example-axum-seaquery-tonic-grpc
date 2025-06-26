use async_trait::async_trait;
use sea_query::{Expr, Func, Order, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use tracing::{error, info};

use crate::abstract_trait::UserRepositoryTrait;
use crate::config::ConnectionPool;
use crate::domain::{CreateUserRequest, UpdateUserRequest};
use crate::model::user::User;
use crate::schema::user::Users;
use crate::utils::AppError;

pub struct UserRepository {
    db_pool: ConnectionPool,
}

impl UserRepository {
    pub fn new(db_pool: ConnectionPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl UserRepositoryTrait for UserRepository {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<User>, i64), AppError> {
        info!(
            "Finding all users - page: {}, page_size: {}, search: {:?}",
            page, page_size, search
        );

        if page <= 0 || page_size <= 0 {
            return Err(AppError::ValidationError(
                "Page and page_size must be positive".to_string(),
            ));
        }

        let offset = (page - 1) * page_size;

        let mut select_query = Query::select();
        select_query
            .columns([
                Users::Id,
                Users::Firstname,
                Users::Lastname,
                Users::Email,
                Users::Password,
            ])
            .from(Users::Table)
            .order_by(Users::Id, Order::Asc)
            .limit(page_size as u64)
            .offset(offset as u64);

        if let Some(term) = &search {
            select_query.and_where(Expr::col(Users::Email).like(format!("{}%", term)));
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);

        let users_result = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let users = match users_result {
            Ok(u) => u,
            Err(e) => {
                error!("Error fetching users: {}", e);
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} users", users.len());

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(Users::Id)))
            .from(Users::Table);

        if let Some(term) = &search {
            count_query.and_where(Expr::col(Users::Email).like(format!("{}%", term)));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok(count) => count.0,
            Err(e) => {
                error!("Error counting users: {}", e);
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} users out of total {}", users.len(), total);

        Ok((users, total))
    }

    async fn find_by_email_exists(&self, email: &str) -> Result<bool, AppError> {
        let query = Query::select()
            .expr(Expr::col(Users::Id).count())
            .from(Users::Table)
            .and_where(Expr::col(Users::Email).eq(email))
            .to_owned();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
        let count: i64 = sqlx::query_scalar_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await?;

        Ok(count > 0)
    }

    async fn create_user(&self, input: &CreateUserRequest) -> Result<User, AppError> {
        let query = Query::insert()
            .into_table(Users::Table)
            .columns([
                Users::Firstname,
                Users::Lastname,
                Users::Email,
                Users::Password,
            ])
            .values_panic([
                input.firstname.clone().into(),
                input.lastname.clone().into(),
                input.email.clone().into(),
                input.password.clone().into(),
            ])
            .returning_all()
            .to_owned();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
        let user: User = sqlx::query_as_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await?;

        Ok(user)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let query = Query::select()
            .columns([
                Users::Id,
                Users::Firstname,
                Users::Lastname,
                Users::Email,
                Users::Password,
            ])
            .from(Users::Table)
            .and_where(Expr::col(Users::Email).eq(email))
            .to_owned();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
        let user = sqlx::query_as_with(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(user)
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<User>, AppError> {
        let query = Query::select()
            .columns([
                Users::Id,
                Users::Firstname,
                Users::Lastname,
                Users::Email,
                Users::Password,
            ])
            .from(Users::Table)
            .and_where(Expr::col(Users::Id).eq(id))
            .to_owned();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
        let user = sqlx::query_as_with(&sql, values)
            .fetch_optional(&self.db_pool)
            .await?;

        Ok(user)
    }

    async fn update_user(&self, input: &UpdateUserRequest) -> Result<User, AppError> {
        let id = input
            .id
            .ok_or_else(|| AppError::ValidationError("User ID is required".into()))?;

        let mut update_query = Query::update();
        let mut query = update_query
            .table(Users::Table)
            .and_where(Expr::col(Users::Id).eq(id));

        if let Some(firstname) = &input.firstname {
            query = query.value(Users::Firstname, firstname.clone());
        }

        if let Some(lastname) = &input.lastname {
            query = query.value(Users::Lastname, lastname.clone());
        }

        if let Some(email) = &input.email {
            query = query.value(Users::Email, email.clone());
        }

        query = query.returning_all();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
        let user = sqlx::query_as_with(&sql, values)
            .fetch_one(&self.db_pool)
            .await?;

        Ok(user)
    }

    async fn delete_user(&self, email: &str) -> Result<(), AppError> {
        let query = Query::delete()
            .from_table(Users::Table)
            .and_where(Expr::col(Users::Email).eq(email))
            .to_owned();

        let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }
}
