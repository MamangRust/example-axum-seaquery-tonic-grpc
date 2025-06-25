use genproto::api::ApiResponseEmpty;
use genproto::category::{
    ApiResponseCategoriesPaginated, ApiResponseCategory, CreateCategoryRequest,
    FindAllCategoryRequest, FindCategoryRequest, UpdateCategoryRequest,
    category_service_server::CategoryService,
};

use shared::{
    domain::{
        CreateCategoryRequest as SharedCreateCategoryRequest,
        FindAllCategoryRequest as SharedFindAllCategoryRequest,
        UpdateCategoryRequest as SharedUpdateCategoryRequest,
    },
    state::AppState,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};
use tracing::{error, info};

pub struct CategoryServiceImpl {
    pub state: Arc<AppState>,
}

impl CategoryServiceImpl {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl CategoryService for CategoryServiceImpl {
    async fn get_categories(
        &self,
        request: Request<FindAllCategoryRequest>,
    ) -> Result<Response<ApiResponseCategoriesPaginated>, Status> {
        info!("Getting categories");

        let req = request.get_ref();

        let myrequest = SharedFindAllCategoryRequest {
            page: req.page,
            page_size: req.page_size,
            search: req.search.clone(),
        };

        match self
            .state
            .di_container
            .category_service
            .get_categories(myrequest)
            .await
        {
            Ok(api_response) => {
                let categories: Vec<_> = api_response.data.into_iter().map(Into::into).collect();

                Ok(Response::new(ApiResponseCategoriesPaginated {
                    status: api_response.status,
                    message: api_response.message,
                    data: categories,
                    pagination: Some(api_response.pagination.into()),
                }))
            }
            Err(err) => {
                error!("Failed to get categories: {}", err.message);
                Err(Status::internal(err.message))
            }
        }
    }

    async fn get_category(
        &self,
        request: Request<FindCategoryRequest>,
    ) -> Result<Response<ApiResponseCategory>, Status> {
        let id = request.into_inner().id;

        match self
            .state
            .di_container
            .category_service
            .get_category(id)
            .await
        {
            Ok(Some(category)) => {
                let reply = ApiResponseCategory {
                    status: "success".into(),
                    message: "Category fetched successfully".into(),
                    data: Some(category.data.into()),
                };
                Ok(Response::new(reply))
            }
            Ok(None) => Err(Status::not_found("Category not found")),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn create_category(
        &self,
        request: Request<CreateCategoryRequest>,
    ) -> Result<Response<ApiResponseCategory>, Status> {
        let req = request.get_ref();

        let body = SharedCreateCategoryRequest {
            name: req.name.clone(),
        };

        match self
            .state
            .di_container
            .category_service
            .create_category(&body)
            .await
        {
            Ok(category) => Ok(Response::new(ApiResponseCategory {
                status: category.status,
                message: category.message,
                data: Some(category.data.into()),
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn update_category(
        &self,
        request: Request<UpdateCategoryRequest>,
    ) -> Result<Response<ApiResponseCategory>, Status> {
        let req = request.get_ref();

        let body = SharedUpdateCategoryRequest {
            id: Some(req.id),
            name: Some(req.name.clone()),
        };

        match self
            .state
            .di_container
            .category_service
            .update_category(&body)
            .await
        {
            Ok(Some(category)) => Ok(Response::new(ApiResponseCategory {
                status: category.status,
                message: category.message,
                data: Some(category.data.into()),
            })),
            Ok(None) => Err(Status::not_found("Category not found")),
            Err(err) => Err(Status::internal(err.message)),
        }
    }

    async fn delete_category(
        &self,
        request: Request<FindCategoryRequest>,
    ) -> Result<Response<ApiResponseEmpty>, Status> {
        let id = request.into_inner().id;

        match self
            .state
            .di_container
            .category_service
            .delete_category(id)
            .await
        {
            Ok(result) => Ok(Response::new(ApiResponseEmpty {
                status: result.status,
                message: result.message,
            })),
            Err(err) => Err(Status::internal(err.message)),
        }
    }
}
