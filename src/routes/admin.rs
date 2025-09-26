use std::{fs, sync::Arc};

use axum::{extract::{Path, State}, http::{StatusCode, HeaderMap}, Json, Router};
// use axum::extract::Multipart;
use axum_extra::extract::Multipart;
use bson::oid::ObjectId;
use uuid::Uuid;
use tower_http::services::ServeDir;

use crate::{models::{category::{Category, CategoryResponse, CreateCategoryMultipart, CreateCategoryResponse, CategoryWithTopUserResponse}, question::{CreateQuestionRequest, CreateQuestionResponse, Question, QuestionResponse}, user::UserResponse}, services::{question_service::QuestionService, quiz_service::QuizService, user_service::UserService}, };



     #[utoipa::path(
    get,
    path = "/admin/categories/{id}/top_user",
    params(
        ("id" = String, Path, description = "Category ID")
    ),
    responses(
        (status = 200, description = "Top user for category", body = UserResponse)
    ),
    security(("bearer_auth" = []))
)]
pub async fn top_user_for_category(
    State((quiz_service, user_service, _question_service)): State<(Arc<QuizService>, Arc<UserService>, Arc<QuestionService>)>,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let category_id = ObjectId::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid category ID".to_string()))?;
    match quiz_service.top_user_for_category(category_id, &user_service).await {
        Ok(Some(user)) => Ok(Json(user)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "No user found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

#[utoipa::path(
    get,
    path = "/admin/categories_with_top_users",
    responses(
        (status = 200, description = "List of categories with top users", body = [CategoryWithTopUserResponse])
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_categories_with_top_users(
    State((quiz_service, user_service, _question_service)): State<(Arc<QuizService>, Arc<UserService>, Arc<QuestionService>)>,
) -> Result<Json<Vec<CategoryWithTopUserResponse>>, (StatusCode, String)> {
    match quiz_service.get_categories_with_top_users(&user_service).await {
        Ok(categories) => Ok(Json(categories)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}



#[utoipa::path(
    post,
    path = "/admin/categories",
    request_body(
        content_type = "multipart/form-data",
        content = inline(CreateCategoryMultipart)
    ),
    responses(
        (status = 201, description = "Category created", body = CreateCategoryResponse),
        (status = 400, description = "Bad request", body = String)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_category(
    State((_quiz_service, _user_service, question_service)): State<(Arc<QuizService>, Arc<UserService>, Arc<QuestionService>)>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<CreateCategoryResponse>), (StatusCode, String)> {
    let mut name = None;
    let mut tags: Option<String> = None;
    let mut parent_id = None;
    let mut image_url = None;

    let base_url = match std::env::var("BASE_URL") {
        Ok(val) => val,
        Err(_) => {
            let host = headers
                .get("host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("localhost:3000"); // Fallback
            let scheme = if host.starts_with("localhost") { "http" } else { "https" };
            format!("{}://{}", scheme, host)
        }
    };

    while let Some(field) = multipart.next_field().await.map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {}", e)))? {
        let field_name = field.name().unwrap().to_string();
        match field_name.as_str() {
            "name" => name = Some(field.text().await.map_err(|e| (StatusCode::BAD_REQUEST, format!("Field error: {}", e)))?),
            "tags" => tags = Some(field.text().await.map_err(|e| (StatusCode::BAD_REQUEST, format!("Field error: {}", e)))?),
            "parent_id" => parent_id = Some(field.text().await.map_err(|e| (StatusCode::BAD_REQUEST, format!("Field error: {}", e)))?),
            "image" => {
                let bytes = field.bytes().await.map_err(|e| (StatusCode::BAD_REQUEST, format!("Field error: {}", e)))?.to_vec();
                // Save image to disk
                let filename = format!("{}.png", Uuid::new_v4());
                let upload_dir = "uploads";
                let filepath = format!("{}/{}", upload_dir, filename);
                fs::create_dir_all(upload_dir).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create upload dir: {}", e)))?;
                fs::write(&filepath, &bytes).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save image: {}", e)))?;
                image_url = Some(format!("{}/uploads/{}", base_url, filename));
            },
            _ => {}
        }
    }

    // Convert parent_id to ObjectId if present
    let parent_oid = match parent_id {
        Some(ref s) if !s.is_empty() => Some(ObjectId::parse_str(s).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid parent_id".to_string()))?),
        _ => None,
    };

let tags_vec = match tags {
    Some(t) if !t.is_empty() => t
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>(),
    _ => Vec::new(), // or return Err if tags are required
};

let category = Category {
    id: None,
    name: name.ok_or((StatusCode::BAD_REQUEST, "Missing name".to_string()))?,
    tags: tags_vec,
    parent_id: parent_oid,
    image_url: image_url.clone(),
    top_user_id: None,
};

    question_service
        .create_category(category.clone())
        .await
        .map(|saved_category_id: String| {
            (StatusCode::CREATED, Json(CreateCategoryResponse {
                message: "Category created successfully".to_string(),
                category: CategoryResponse {
                    id: Some(saved_category_id),
                    name: category.name,
                    tags: category.tags,
                    parent_id: category.parent_id.map(|oid| oid.to_hex()),
                    image_url,
                },
            }))
        })
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

#[utoipa::path(
    get,
    path = "/admin/categories/{id}",
    params(
        ("id" = String, Path, description = "Category ID")
    ),
    responses(
        (status = 200, description = "Category found", body = CategoryResponse),
        (status = 404, description = "Category not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_category(
    State((_quiz_service, _user_service, question_service)): State<(Arc<QuizService>, Arc<UserService>, Arc<QuestionService>)>,
    Path(id): Path<String>,
) -> Result<Json<Category>, (StatusCode, String)> {
    let category_id = ObjectId::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid category ID".to_string()))?;
    question_service.get_category(category_id).await.map(Json).map_err(|e| (StatusCode::NOT_FOUND, e))
}

#[utoipa::path(
    get,
    path = "/admin/categories",
    responses(
        (status = 200, description = "List of categories", body = [CategoryResponse])
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_categories(
    State((_quiz_service, _user_service, question_service)): State<(Arc<QuizService>, Arc<UserService>, Arc<QuestionService>)>,
) -> Result<Json<Vec<Category>>, (StatusCode, String)> {
    question_service.list_categories().await.map(Json).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

#[utoipa::path(
    delete,
    path = "/admin/categories/{id}",
    params(
        ("id" = String, Path, description = "Category ID")
    ),
    responses(
        (status = 204, description = "Category deleted"),
        (status = 404, description = "Category not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_category(
    State((_quiz_service, _user_service, question_service)): State<(Arc<QuizService>, Arc<UserService>, Arc<QuestionService>)>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let category_id = ObjectId::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid category ID".to_string()))?;
    question_service.delete_category(category_id).await.map(|_| StatusCode::NO_CONTENT).map_err(|e| (StatusCode::NOT_FOUND, e))
}

#[utoipa::path(
    post,
    path = "/admin/questions",
    request_body  = CreateQuestionRequest ,
    responses(
        (status = 201, description = "Question created", body = String)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_question(
    State((_quiz_service, _user_service, question_service)): State<(Arc<QuizService>, Arc<UserService>, Arc<QuestionService>)>, 
    Json(question): Json<Question>
) -> Result<(StatusCode, Json<CreateQuestionResponse>), (StatusCode, String)> {
    question_service.create_question(question.clone()).await.map(|_| {
        (StatusCode::CREATED, Json(CreateQuestionResponse {
            message: "Question created successfully".to_string(),
            question: QuestionResponse {
                id: question.id.map(|oid| oid.to_hex()),
                category_id: question.category_id.to_hex(),
                text: question.question,
                question_type: question.question_type,
                options: question.options,
                correct_answer: question.correct_answer,
                explanation: question.explanation,
                difficulty: question.difficulty,
                timer_secs: question.timer.num_seconds(),
                tags: question.tags,
            }
        }))
    })
    .map_err(|e| (StatusCode::BAD_REQUEST, e))
}

#[utoipa::path(
    get,
    path = "/admin/questions/{id}",
    params(
        ("id" = String, Path, description = "Question ID")
    ),
    responses(
        (status = 200, description = "Question found", body = QuestionResponse),
        (status = 404, description = "Question not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_question(
    State((_quiz_service, _user_service, question_service)): State<(Arc<QuizService>, Arc<UserService>, Arc<QuestionService>)>,
    Path(id): Path<String>,
) -> Result<Json<Question>, (StatusCode, String)> {
    let question_id = ObjectId::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid question ID".to_string()))?;
    question_service.get_question(question_id).await.map(Json).map_err(|e| (StatusCode::NOT_FOUND, e))
}

#[utoipa::path(
    get,
    path = "/admin/questions",
    params(
        ("category_id" = Option<String>, Query, description = "Category ID to filter questions")
    ),
    responses(
        (status = 200, description = "List of questions", body = [QuestionResponse])
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_questions(
    State((_quiz_service, _user_service, question_service)): State<(Arc<QuizService>, Arc<UserService>, Arc<QuestionService>)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<Question>>, (StatusCode, String)> {
    let category_id = params.get("category_id").and_then(|id| ObjectId::parse_str(id).ok());
    question_service.list_questions(category_id).await.map(Json).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

#[utoipa::path(
    delete,
    path = "/admin/questions/{id}",
    params(
        ("id" = String, Path, description = "Question ID")
    ),
    responses(
        (status = 204, description = "Question deleted"),
        (status = 404, description = "Question not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_question(
    State((_quiz_service, _user_service, question_service)): State<(Arc<QuizService>, Arc<UserService>, Arc<QuestionService>)>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let question_id = ObjectId::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid question ID".to_string()))?;
    question_service.delete_question(question_id).await.map(|_| StatusCode::NO_CONTENT).map_err(|e| (StatusCode::NOT_FOUND, e))
}




pub fn admin_routes(
    question_service: Arc<QuestionService>,
    quiz_service: Arc<QuizService>,
    user_service: Arc<UserService>,
) -> Router {
    let state = (quiz_service, user_service, question_service.clone());

    Router::new()
        .route(
            "/admin/categories",
            axum::routing::post(create_category).get(list_categories),
        )
        .route("/admin/categories/{id}", axum::routing::get(get_category).delete(delete_category))
        .route("/admin/questions", axum::routing::post(create_question).get(list_questions))
        .route("/admin/questions/{id}", axum::routing::get(get_question).delete(delete_question))
        .nest_service("/uploads", ServeDir::new("uploads"))
        .route("/admin/categories/{id}/top_user", axum::routing::get(top_user_for_category))
        .route("/admin/categories_with_top_users", axum::routing::get(get_categories_with_top_users))
        .with_state(state)
}
