mod auth;
mod category;
mod comment;
mod file;
mod posts;
mod user;

pub use self::auth::AuthService;
pub use self::category::CategoryService;
pub use self::comment::CommentService;
pub use self::file::FileService;
pub use self::posts::PostService;
pub use self::user::UserService;
