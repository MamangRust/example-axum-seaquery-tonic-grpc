mod auth;
mod category;
mod comment;
mod file;
mod post;
mod user;

pub use self::category::{
    CategoryRepositoryTrait, CategoryServiceTrait, DynCategoryRepository, DynCategoryService,
};

pub use self::post::{
    DynPostsRepository, DynPostsService, PostsRepositoryTrait, PostsServiceTrait,
};

pub use self::comment::{
    CommentRepositoryTrait, CommentServiceTrait, DynCommentRepository, DynCommentService,
};

pub use self::user::{DynUserRepository, DynUserService, UserRepositoryTrait, UserServiceTrait};

pub use self::auth::{AuthServiceTrait, DynAuthService};

pub use self::file::{DynFileService, FileServiceTrait};
