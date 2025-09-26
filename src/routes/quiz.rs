use axum::{
    extract::{Path, State}, http::StatusCode, Extension, Json, Router
};
use bson::oid::ObjectId;
use serde::Deserialize;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{middleware::auth::auth_middleware, models::quiz::QuizResponse, utils::Claims};
use crate::models::question::Difficulty;
use crate::models::quiz::Quiz;
use crate::services::{quiz_service::QuizService, user_service::UserService, };
use crate::models::leaderboard::LeaderboardEntry;

#[derive(Deserialize, ToSchema)]
pub struct StartQuizRequest {
    category_id: String,
    difficulty: Difficulty,
    num_questions: i32,
}

#[derive(Deserialize, ToSchema)]
pub struct SubmitAnswerRequest {
    question_id: String,
    answer: String,
    time_taken: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct PauseQuizRequest {
    paused: bool,
}

#[utoipa::path(
    get,
    path = "/quiz/leaderboard/{category_id}",
    params(
        ("category_id" = String, Path, description = "Category ID")
    ),
    responses(
        (status = 200, description = "Leaderboard retrieved successfully", body = [LeaderboardEntry]),
        (status = 400, description = "Invalid category ID"),
    )
)]
pub async fn get_leaderboard(
    State((quiz_service, _user_service)): State<(Arc<QuizService>, Arc<UserService>)>,
    Path(category_id): Path<String>,
) -> Result<Json<Vec<crate::models::leaderboard::LeaderboardEntry>>, (StatusCode, String)> {
    let category_id = ObjectId::parse_str(&category_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid category ID".to_string()))?;

    let leaderboard = quiz_service.leaderboard_service
        .get_leaderboard(category_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(leaderboard))
}

#[utoipa::path(
    post,
    path = "/quiz/start",
    request_body = StartQuizRequest,
    responses(
        (status = 201, description = "Quiz started successfully", body = QuizResponse),
        (status = 400, description = "Invalid request or not enough questions"),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn start_quiz(
    State((quiz_service, _user_service)): State<(Arc<QuizService>, Arc<UserService>)>,
    Extension(claims): Extension<Arc<Claims>>,    // 👈 this brings `claims` into scope
    Json(req): Json<StartQuizRequest>,
) -> Result<(StatusCode, Json<QuizResponse>), (StatusCode, String)> {
    let user_id = ObjectId::parse_str(&claims.sub)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let category_id = ObjectId::parse_str(&req.category_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid category ID".to_string()))?;

    let quiz = quiz_service
        .start_quiz(user_id, category_id, req.difficulty, req.num_questions)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok((StatusCode::CREATED, Json(quiz.into())))
}

#[utoipa::path(
    post,
    path = "/quiz/{id}/answer",
    params(
        ("id" = String, Path, description = "Quiz ID")
    ),
    request_body = SubmitAnswerRequest,
    responses(
        (status = 200, description = "Answer submitted successfully", body = SubmitAnswerRequest),
        (status = 400, description = "Invalid request or quiz paused"),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn submit_answer(
    State((quiz_service, _user_service)): State<(Arc<QuizService>, Arc<UserService>)>,
    Path(id): Path<String>,
    Json(req): Json<SubmitAnswerRequest>,
) -> Result<(StatusCode, Json<Quiz>), (StatusCode, String)> {
    let quiz_id = ObjectId::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid quiz ID".to_string()))?;
    let question_id = ObjectId::parse_str(&req.question_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid question ID".to_string()))?;

    let quiz = quiz_service
        .submit_answer(quiz_id, question_id, req.answer, req.time_taken)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok((StatusCode::OK, Json(quiz)))
}

#[utoipa::path(
    post,
    path = "/quiz/{id}/finish",
    params(
        ("id" = String, Path, description = "Quiz ID")
    ),
    responses(
        (status = 200, description = "Quiz finished successfully", body = i32),
        (status = 400, description = "Invalid request or quiz not found"),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn finish_quiz(
    State((quiz_service, user_service)): State<(Arc<QuizService>, Arc<UserService>)>,
    Extension(_claims): Extension<Arc<Claims>>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<i32>), (StatusCode, String)> {
    let quiz_id = ObjectId::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid quiz ID".to_string()))?;

    let score = quiz_service
        .finish_quiz(&user_service, quiz_id)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok((StatusCode::OK, Json(score)))
}

#[utoipa::path(
    post,
    path = "/quiz/{id}/pause",
    params(
        ("id" = String, Path, description = "Quiz ID")
    ),
    request_body = PauseQuizRequest,
    responses(
        (status = 200, description = "Quiz paused/resumed successfully", body = QuizResponse),
        (status = 400, description = "Invalid request or quiz not found"),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn pause_quiz(
    State((quiz_service, _user_service)): State<(Arc<QuizService>, Arc<UserService>)>,
    Path(id): Path<String>,
    Json(req): Json<PauseQuizRequest>,
) -> Result<(StatusCode, Json<QuizResponse>), (StatusCode, String)> {
    let quiz_id = ObjectId::parse_str(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid quiz ID".to_string()))?;

    let quiz = quiz_service
        .pause_quiz(quiz_id, req.paused)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    Ok((StatusCode::OK, Json(quiz.into())))
}

pub fn quiz_routes(quiz_service: Arc<QuizService>, user_service: Arc<UserService>) -> Router {
    Router::new()
        .route("/quiz/start", axum::routing::post(start_quiz))
        .route("/quiz/{id}/answer", axum::routing::post(submit_answer))
        .route("/quiz/{id}/finish", axum::routing::post(finish_quiz))
        .route("/quiz/{id}/pause", axum::routing::post(pause_quiz))
        .route("/quiz/leaderboard/{category_id}", axum::routing::get(get_leaderboard))

    .layer(axum::middleware::from_fn(auth_middleware))
        .with_state((quiz_service, user_service))
}