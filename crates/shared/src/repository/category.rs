use crate::abstract_trait::CategoryRepositoryTrait;
use crate::config::ConnectionPool;
use crate::domain::{CreateCategoryRequest, UpdateCategoryRequest};
use crate::model::category::Category;
use crate::schema::category::Categories;
use crate::utils::AppError;
use async_trait::async_trait;
use sea_query::{Expr, Func, Order, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use tracing::{debug, error, info};

pub struct CategoryRepository {
    db_pool: ConnectionPool,
}

impl CategoryRepository {
    pub fn new(db_pool: ConnectionPool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl CategoryRepositoryTrait for CategoryRepository {
    async fn find_all(
        &self,
        page: i32,
        page_size: i32,
        search: Option<String>,
    ) -> Result<(Vec<Category>, i64), AppError> {
        info!(
            "Finding all categories - page: {}, page_size: {}, search: {:?}",
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
            .columns([Categories::Id, Categories::Name])
            .from(Categories::Table)
            .order_by(Categories::Id, Order::Asc)
            .limit(page_size as u64)
            .offset(offset as u64);

        if let Some(term) = &search {
            select_query.and_where(Expr::col(Categories::Name).like(format!("{}%", term)));
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);

        let categories_result = sqlx::query_as_with::<_, Category, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let categories = match categories_result {
            Ok(cats) => cats,
            Err(e) => {
                error!("Error fetching categories: {}", e);
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} categories", categories.len());

        let mut count_query = Query::select();
        count_query
            .expr(Func::count(Expr::col(Categories::Id)))
            .from(Categories::Table);

        if let Some(term) = &search {
            count_query.and_where(Expr::col(Categories::Name).like(format!("{}%", term)));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);

        debug!("Count SQL: {}", count_sql);

        let total_result = sqlx::query_as_with::<_, (i64,), _>(&count_sql, count_values)
            .fetch_one(&self.db_pool)
            .await;

        let total = match total_result {
            Ok(count) => count.0,
            Err(e) => {
                error!("Error counting categories: {}", e);
                return Err(AppError::SqlxError(e));
            }
        };

        info!(
            "Found {} categories out of total {}",
            categories.len(),
            total
        );

        Ok((categories, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Category>, AppError> {
        info!("Finding category by id: {}", id);

        let query = Query::select()
            .columns([Categories::Id, Categories::Name])
            .from(Categories::Table)
            .and_where(Expr::col(Categories::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = query;

        match sqlx::query_as_with::<_, Category, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
        {
            Ok(result) => {
                info!("Find result: {:?}", result);
                Ok(result)
            }
            Err(e) => {
                error!("Error fetching category by ID {}: {}", id, e);
                Err(AppError::SqlxError(e))
            }
        }
    }

    async fn create(&self, input: &CreateCategoryRequest) -> Result<Category, AppError> {
        info!("Creating new category: {:?}", input.name);

        let insert = Query::insert()
            .into_table(Categories::Table)
            .columns([Categories::Name])
            .values_panic([input.name.clone().into()])
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = insert;

        let result = sqlx::query_as_with::<_, Category, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::SqlxError)?;

        info!("New category inserted with ID: {}", result.id);

        Ok(result)
    }

    async fn update(&self, input: &UpdateCategoryRequest) -> Result<Category, AppError> {
        let id = input
            .id
            .ok_or(AppError::ValidationError("ID is required".into()))?;

        info!("Updating category ID: {} with name: {:?}", id, input.name);

        let update = Query::update()
            .table(Categories::Table)
            .values(vec![(
                Categories::Name,
                input.name.clone().unwrap_or_default().into(),
            )])
            .and_where(Expr::col(Categories::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = update;

        let res = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await?;

        if res.rows_affected() == 0 {
            info!("No category found to update with ID: {}", id);
            return Err(AppError::SqlxError(sqlx::Error::RowNotFound));
        }

        info!("Category ID: {} updated successfully", id);

        self.find_by_id(id)
            .await?
            .ok_or(AppError::SqlxError(sqlx::Error::RowNotFound))
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        info!("Deleting category with ID: {}", id);

        let delete = Query::delete()
            .from_table(Categories::Table)
            .and_where(Expr::col(Categories::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = delete;

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await?;

        if result.rows_affected() == 0 {
            info!("No category found to delete with ID: {}", id);
            return Err(AppError::SqlxError(sqlx::Error::RowNotFound));
        }

        info!("Category ID: {} deleted successfully", id);
        Ok(())
    }
}
