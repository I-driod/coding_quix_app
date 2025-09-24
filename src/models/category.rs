use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use base64;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use super::user::UserResponse;

/// DB model (keep as-is, used for MongoDB)
#[derive(Serialize, Deserialize, Clone)]
pub struct Category {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub name: String,
    pub tags: Vec<String>,
    pub parent_id: Option<ObjectId>,
    pub image_url: Option<String>,
    pub top_user_id: Option<ObjectId>,
}



/// Request DTO (what clients POST)
#[derive(Deserialize, Clone, ToSchema)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
}

/// Response DTO (what the API returns)
#[derive(Serialize, ToSchema, Clone)]
pub struct CategoryResponse {
    pub id: Option<String>,
    pub name: String,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    pub image_url: Option<String>,



}

/// Response DTO for a category with its top scoring user
#[derive(Serialize, ToSchema, )]
pub struct CategoryWithTopUserResponse {
    #[serde(flatten)]
    pub category: CategoryResponse,
    pub top_user: Option<UserResponse>,
}

/// Response wrapper for create category
#[derive(Serialize, ToSchema)]
pub struct CreateCategoryResponse {
    pub message: String,
    pub category: CategoryResponse,
}

impl From<(Category, Option<String>)> for CategoryResponse {
    fn from((c, image_url): (Category, Option<String>)) -> Self {
        CategoryResponse {
            id: c.id.map(|oid| oid.to_hex()),
            name: c.name, 
            image_url: c.image_url,
            parent_id: c.parent_id.map(|oid| oid.to_hex()),
            tags: c.tags,
            // top_user is handled separately in the service layer
       
        }
    }
}

impl From<CreateCategoryRequest> for Category {
    fn from(req: CreateCategoryRequest) -> Self {
        Category {
            id: None,
            name: req.name,
            tags: req.tags,
            parent_id: req.parent_id.and_then(|id_str| ObjectId::parse_str(&id_str).ok()),
            image_url: None,
            top_user_id: None,
        }
    }
}


// #[derive(utoipa::ToSchema)]
// pub struct CreateCategoryMultipart {
//     pub name: String,
//     pub tags: Vec<String>,
//     pub parent_id: Option<String>,
//     #[schema(format = "binary")]
//     pub image: Option<String>,
// }

#[derive(ToSchema)]
pub struct CreateCategoryMultipart {
    pub name: String,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    #[schema(value_type = String, format = "binary")]
    pub image: Option<Vec<u8>>,
}
