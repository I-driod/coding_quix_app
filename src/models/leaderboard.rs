use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct LeaderboardEntry {
    #[schema(value_type = String)]
    pub user_id: ObjectId,
    #[schema(value_type = String)]
    pub category_id: ObjectId,
    pub score: i32,
    pub rank: i32,
}
