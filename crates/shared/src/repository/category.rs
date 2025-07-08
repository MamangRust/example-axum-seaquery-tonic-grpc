use crate::abstract_trait::CategoryRepositoryTrait;
use crate::config::ConnectionPool;
use crate::domain::{CreateCategoryRequest, UpdateCategoryRequest};
use crate::model::category::Category;
use crate::schema::category::Categories;
use crate::utils::AppError;
use anyhow::Result;
use async_trait::async_trait;
use sea_query::{Expr, Func, Order, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use tracing::{error, info};

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
            "Getting all categories - page: {page}, page_size: {page_size}, search: {:?}",
            search
        );

        let page = if page > 0 { page } else { 1 };
        let page_size = if page_size > 0 { page_size } else { 10 };

        let offset = (page - 1) * page_size;

        let mut select_query = Query::select();
        select_query
            .columns([Categories::Id, Categories::Name])
            .from(Categories::Table)
            .order_by(Categories::Id, Order::Asc)
            .limit(page_size as u64)
            .offset(offset as u64);

        if let Some(term) = &search {
            select_query.and_where(Expr::col(Categories::Name).like(format!("{term}%")));
        }

        let (sql, values) = select_query.build_sqlx(PostgresQueryBuilder);

        let categories_result = sqlx::query_as_with::<_, Category, _>(&sql, values)
            .fetch_all(&self.db_pool)
            .await;

        let categories = match categories_result {
            Ok(cats) => cats,
            Err(e) => {
                error!("Error fetching categories: {e}");
                return Err(AppError::SqlxError(e));
            }
        };

        info!("Found {} categories", categories.len());

        let mut count_query = Query::select();

        count_query
            .expr(Func::count(Expr::col(Categories::Id)))
            .from(Categories::Table);

        if let Some(term) = &search {
            count_query.and_where(Expr::col(Categories::Name).like(format!("{term}%")));
        }

        let (count_sql, count_values) = count_query.build_sqlx(PostgresQueryBuilder);

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

        info!("Found {} categories out of total {total}", categories.len(),);

        Ok((categories, total))
    }

    async fn find_by_id(&self, id: i32) -> Result<Option<Category>, AppError> {
        info!("Finding category by id: {id}");

        let (sql, values) = Query::select()
            .columns([Categories::Id, Categories::Name])
            .from(Categories::Table)
            .and_where(Expr::col(Categories::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_as_with::<_, Category, _>(&sql, values)
            .fetch_optional(&self.db_pool)
            .await
            .map_err(AppError::from)?;

        info!("Find result: {:?}", result);
        Ok(result)
    }

    async fn create(&self, input: &CreateCategoryRequest) -> Result<Category, AppError> {
        info!("Creating new category: {:?}", input.name);

        let insert = Query::insert()
            .into_table(Categories::Table)
            .columns([Categories::Name])
            .values([input.name.clone().into()])
            .unwrap()
            .to_owned()
            .build_sqlx(PostgresQueryBuilder);

        let (sql, values) = insert;

        let result = sqlx::query_as_with::<_, Category, _>(&sql, values)
            .fetch_one(&self.db_pool)
            .await
            .map_err(AppError::from)?;

        info!("New category inserted with ID: {}", result.id);

        Ok(result)
    }

    async fn update(&self, input: &UpdateCategoryRequest) -> Result<Category, AppError> {
        info!(
            "Updating category ID {} with new name '{}'",
            input.id, input.name
        );

        let (sql, values) = Query::update()
            .table(Categories::Table)
            .values([(Categories::Name, Expr::val(input.name.clone()).into())])
            .and_where(Expr::col(Categories::Id).eq(input.id))
            .build_sqlx(PostgresQueryBuilder);

        let affected = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await?
            .rows_affected();

        if affected == 0 {
            error!("Category ID {} not found for update", input.id);
            return Err(AppError::NotFound(format!(
                "Category with ID {} not found",
                input.id
            )));
        }

        let category = self.find_by_id(input.id).await?.ok_or_else(|| {
            AppError::NotFound(format!("Category with ID {} not found", input.id))
        })?;

        info!("Successfully updated category ID {}", input.id);
        Ok(category)
    }

    async fn delete(&self, id: i32) -> Result<(), AppError> {
        info!("Deleting category with ID: {id}");

        let (sql, values) = Query::delete()
            .from_table(Categories::Table)
            .and_where(Expr::col(Categories::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_with(&sql, values)
            .execute(&self.db_pool)
            .await?;

        match result.rows_affected() {
            0 => {
                error!("No category found to delete with ID: {id}");
                Err(AppError::NotFound(format!(
                    "Category with ID {id} not found"
                )))
            }
            _ => {
                info!("Category ID: {id} deleted successfully");
                Ok(())
            }
        }
    }
}
