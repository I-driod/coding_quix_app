use axum::{extract::State, http::StatusCode, Json, Router};
use serde::Deserialize;
use utoipa::ToSchema;
use crate::{config, services::user_service::{LoginResponse, UserService}};
use std::sync::Arc;
use crate::models::user::Role;



#[derive(Deserialize, ToSchema)]
pub struct StartVerificationRequest {
    phone_number: String, // E.164 recommended
}




#[derive(Deserialize, ToSchema)]
pub struct RegisterRequest {
    phone_number: i32,
    username: String,
    password: String,
    role: Option<String>,  // "user" or "admin", default user
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    phone_number: String,
    password: String,
}

// #[utoipa::path(
//     post,
//     path = "/register",
//     request_body = RegisterRequest,
//     responses(
//         (status = 201, description = "User registered")
//     )
// )]
// pub async fn register(State((user_service, config)): State<(Arc<UserService>, Arc<config::Config>)>, Json(req): Json<RegisterRequest>) -> Result<StatusCode, (StatusCode, String)> {
//     let role = match req.role.as_deref() {
//         Some("admin") => Role::Admin,
//         _ => Role::User,
//     };
//     user_service.register(req.phone_number, req.username, req.password, role).await.map(|_| StatusCode::CREATED).map_err(|e| (StatusCode::BAD_REQUEST, e))
// }

#[utoipa::path(
    post,
    path = "/login",
    request_body = LoginRequest,
    responses(
    (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Unauthorized", body = LoginResponse)
    )
)]
pub async fn login(
    State((user_service, config)): State<(Arc<UserService>, Arc<config::Config>)>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<LoginResponse>)> {
    // map the service error into our (StatusCode, Json<LoginResponse>) error type,
    // then `?` to early-return on error.
    let (user, token) = user_service
        .login(req.phone_number, req.password, &config.jwt_secret)
        .await
        .map_err(|e| {
            let body = LoginResponse {
                message: e,
                token: None,
                user: None,
            };
            (StatusCode::UNAUTHORIZED, Json(body))
        })?; // <-- important

    let body = LoginResponse {
        message: "Login successful".to_string(),
        token: Some(token),
        user: Some(user),
    };

    Ok(Json(body))
}

#[derive(Deserialize, ToSchema)]
pub struct ConfirmRegisterRequest {
    phone_number: String,
    code: String,
    username: String,
    password: String,
    role: Option<String>,
}

#[utoipa::path(
    post,
    path = "/start_verification",
    request_body = StartVerificationRequest,
    responses(
        (status = 200, description = "OTP sent")
    )
)]

pub async fn start_verification(
    State((user_service, _config)): State<(Arc<UserService>, Arc<config::Config>)>,
    Json(req): Json<StartVerificationRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // optionally validate/normalize the phone here (phonenumber crate)
    user_service.start_phone_verification(req.phone_number).await
        .map(|_| StatusCode::OK)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

#[utoipa::path(
    post,
    path = "/confirm_register",
    request_body = ConfirmRegisterRequest,
    responses(
        (status = 201, description = "User registered")
    )
)]
pub async fn confirm_register(
    State((user_service, _config)): State<(Arc<UserService>, Arc<config::Config>)>,
    Json(req): Json<ConfirmRegisterRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let role = match req.role.as_deref() {
        Some("admin") => Role::Admin,
        _ => Role::User,
    };

    user_service.verify_and_register(req.phone_number, req.code, req.username, req.password, role).await
        .map(|_| StatusCode::CREATED)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))
}


pub fn auth_routes(user_service: Arc<UserService>, config: Arc<config::Config>) -> Router {
    Router::new()
        .route("/start_verification", axum::routing::post(start_verification))
        .route("/confirm_register", axum::routing::post(confirm_register))
        .route("/login", axum::routing::post(login))
        .with_state((user_service, config))
}
