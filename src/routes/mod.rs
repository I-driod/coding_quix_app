

use axum::{Router, routing::get};
use std::sync::Arc;
use mongodb::Database;

use crate::{config, services::{phone_verify::TwilioClient, leaderboard_service::LeaderboardService}};

pub mod auth;
pub mod user;
pub mod admin;
pub mod quiz;

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Health check OK")
    )
)]
async fn health_check() -> &'static str {
    "OK"
}

pub fn init_routes(db: Arc<Database>, config: Arc<config::Config>) -> Router {
    use crate::services::user_service::UserService;
  let twilio = Arc::new(TwilioClient::new_from_env());
    let question_service = Arc::new(crate::services::question_service::QuestionService::new(db.clone()));
    let leaderboard_service = Arc::new(LeaderboardService::new(db.clone()));

    let user_service = Arc::new(UserService::new(db.clone(), twilio));
    let quiz_service = Arc::new(crate::services::quiz_service::QuizService::new(db.clone(), leaderboard_service.clone()));

    Router::new()
        .route("/health", get(health_check))
        .merge(auth::auth_routes(user_service.clone(), config.clone()))
        .merge(user::user_routes(user_service.clone()))
        .merge(admin::admin_routes(
            question_service.clone(),
            quiz_service.clone(),
            user_service.clone()
        ))
        .merge(quiz::quiz_routes(quiz_service, user_service.clone()))
}
