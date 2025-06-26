mod auth;
mod category;
mod comments;
mod posts;
mod user;

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use prometheus_client::encoding::text::encode;
use tokio::net::TcpListener;
use tower_http::limit::RequestBodyLimitLayer;
use utoipa::openapi::security::SecurityScheme;
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

pub use self::auth::auth_routes;
pub use self::category::category_routes;
pub use self::comments::comment_routes;
pub use self::posts::post_routes;
pub use self::user::user_routes;

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login_user_handler,
        auth::get_me_handler,
        auth::register_user_handler,
        user::get_users,
        user::get_user,
        user::create_user,
        user::update_user,
        user::delete_user,
        category::get_categories,
        category::get_category,
        category::create_category,
        category::update_category,
        category::delete_category,
        comments::get_comments,
        comments::get_comment,
        comments::create_comment,
        comments::update_comment,
        comments::delete_comment,
        posts::get_posts,
        posts::get_post,
        posts::get_post_relation,
        posts::create_post,
        posts::update_post,
        posts::delete_post,
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "Authentication endpoints."),
        (name = "category", description = "Category management endpoints."),
        (name = "posts", description = "Post management endpoints."),
        (name = "comments", description = "Comments management endpoints."),
        (name = "users", description = "User management endpoints.")
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();

        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(utoipa::openapi::security::Http::new(
                utoipa::openapi::security::HttpAuthScheme::Bearer,
            )),
        );
    }
}

pub async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut buffer = String::new();
    encode(&mut buffer, &state.registry).unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .header(
            CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )
        .body(Body::from(buffer))
        .unwrap()
}

pub struct AppRouter;

impl AppRouter {
    pub async fn serve(port: u16, app_state: AppState) -> Result<(), Box<dyn std::error::Error>> {
        let shared_state = Arc::new(app_state);

        let mut router = OpenApiRouter::with_openapi(ApiDoc::openapi())
            .route("/metrics", get(metrics_handler))
            .with_state(shared_state.clone());

        router = router.merge(auth_routes(shared_state.clone()));
        router = router.merge(category_routes(shared_state.clone()));
        router = router.merge(comment_routes(shared_state.clone()));
        router = router.merge(post_routes(shared_state.clone()));
        router = router.merge(user_routes(shared_state.clone()));

        let router = router
            .layer(DefaultBodyLimit::disable())
            .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024));

        let (router, api) = router.split_for_parts();

        let router =
            router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()));

        let addr = format!("0.0.0.0:{port}");
        let listener = TcpListener::bind(addr).await?;
        println!("Server running on http://{}", listener.local_addr()?);

        axum::serve(listener, router).await?;
        Ok(())
    }
}
