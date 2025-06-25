use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use genproto::user::UserResponse as ProtoUserResponse;
use crate::model::user::User;


#[derive(Debug, Deserialize, Serialize,  Clone, ToSchema)]
pub struct UserResponse {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            firstname: user.firstname,
            lastname: user.lastname,
            email: user.email,
        }
    }
}

impl From<UserResponse> for ProtoUserResponse {
    fn from(user: UserResponse) -> Self {
        Self {
            id: user.id,
            firstname: user.firstname,
            lastname: user.lastname,
            email: user.email,
        }
    }
}

impl From<ProtoUserResponse> for UserResponse {
    fn from(user: ProtoUserResponse) -> Self {
        Self {
            id: user.id,
            firstname: user.firstname,
            lastname: user.lastname,
            email: user.email,
        }
    }
}