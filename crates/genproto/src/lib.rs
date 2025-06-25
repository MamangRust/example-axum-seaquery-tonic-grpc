pub mod api{
    include!("gen/api.rs");
}

pub mod auth{
    include!("gen/auth.rs");
}

pub mod user{
    include!("gen/user.rs");
}

pub mod comment{
    include!("gen/comment.rs");
}

pub mod category{
    include!("gen/category.rs");
}

pub mod post{
    include!("gen/post.rs");
}