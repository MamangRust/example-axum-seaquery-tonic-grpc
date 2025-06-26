mod auth;
mod category;
mod comment;
mod posts;
mod user;

pub use self::auth::{AuthServiceTrait, DynAuthService};
pub use self::category::{CategoryServiceTrait, DynCategoryService};
pub use self::comment::{CommentServiceTrait, DynCommentService};
pub use self::posts::{DynPostsService, PostsServiceTrait};
pub use self::user::{DynUserService, UserServiceTrait};
