use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;




#[derive(Serialize, ToSchema)]
pub struct UserResponse {
    pub id: Option<String>,
    pub phone_number: String,
    pub username: String,
    pub role: Role,
    pub profile: Profile,
    pub xp: i32,
    pub quiz_history: Vec<String>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        UserResponse {
            id: u.id.map(|oid| oid.to_hex()),   // ObjectId -> hex string
            phone_number: u.phone_number,
            username: u.username,
            role: u.role,
            profile: u.profile,
            xp: u.xp,
            quiz_history: u.quiz_history.into_iter().map(|uuid| uuid.to_string()).collect(),
        }
    }
}





#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub phone_number: String,
    pub username: String,
    pub password_hash: String,
    pub role: Role,
    pub profile: Profile,
    pub xp: i32,
    pub quiz_history: Vec<Uuid>
}


#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub enum Role {
    User,
    Admin,
}



#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Profile {
    pub avatar: Option<String>,
    pub bio: Option<String>,
    pub preferred_language: Option<String>,
    pub country: Option<String>,
}
