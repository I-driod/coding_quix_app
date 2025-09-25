use axum::{extract::{Path, State}, http::StatusCode, middleware, Json, Router};
use bson::oid::ObjectId;
use serde::Deserialize;
use utoipa::ToSchema;
use crate::models::user::{Profile, UserResponse};
use crate::services::user_service::UserService;
use crate::middleware::auth::auth_middleware;
use std::sync::Arc;

#[derive(Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    profile: Profile,
}

#[utoipa::path(
    put,
    path = "/user/{id}/profile",
    request_body = UpdateProfileRequest
)]

pub async fn update_profile(State(user_service): State<Arc<UserService>>, Path(id): Path<String>, Json(req): Json<UpdateProfileRequest>) -> Result<StatusCode, (StatusCode, String)> {
    let user_id = ObjectId::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid ID".to_string()))?;
    user_service.update_profile(user_id, req.profile).await.map(|_| StatusCode::OK).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

#[utoipa::path(
    get,
    path = "/user/{id}",
    responses(
        (status = 200, description = "User found", body = UserResponse),
        (status = 404, description = "User not found")
    )
)]
pub async fn get_user(
    State(user_service): State<Arc<UserService>>, 
    Path(id): Path<String>
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let user_id = ObjectId::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid ID".to_string()))?;
    
    user_service
        .get_user(user_id)
        .await
        .map(|user| Json(user.into()))
        .map_err(|e| (StatusCode::NOT_FOUND, e))
}


pub fn user_routes(user_service: Arc<UserService>) -> Router {
    Router::new()
        .route("/user/{id}/profile", axum::routing::put(update_profile))
        .route("/user/{id}", axum::routing::get(get_user))
        .layer(middleware::from_fn(auth_middleware))
        .with_state(user_service)
}
