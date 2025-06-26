mod auth;
mod category;
mod comment;
mod posts;
mod user;

pub use self::auth::AuthService;
pub use self::category::CategoryService;
pub use self::comment::CommentService;
pub use self::posts::PostsService;
pub use self::user::UserService;
