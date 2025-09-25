#[cfg(test)]
mod tests {
    use axum::{
        body,
        http::{Request, StatusCode},
    };
    use bson::oid::ObjectId;
    use serde_json::json;
    use tower::ServiceExt;

    use crate::models::{question::Difficulty, user::Role};
    use crate::services::quiz_service::QuizService;

    use super::super::common::{create_test_user, json_body, setup};

    #[tokio::test]
    async fn test_finish_quiz_endpoint() {
        // Arrange
        let ctx = setup().await;
        let user = create_test_user(&ctx.db, Role::User, "quizuser", "+15550001111").await;

        // 1. Create a quiz directly via the service
        let category_id = ObjectId::new(); // Using a mock category ID for this test
        let quiz_service = QuizService::new(ctx.db.clone());
        let start_quiz_result = quiz_service
            .start_quiz(user.id, category_id, Difficulty::Beginner, 5)
            .await;

        assert!(
            start_quiz_result.is_ok(),
            "Test setup failed: Could not start quiz"
        );
        let quiz = start_quiz_result.unwrap();
        let quiz_id = quiz.id.expect("Quiz must have an ID after creation");

        // 2. Call the /quiz/{id}/finish endpoint
        let finish_url = format!("/quiz/{}/finish", quiz_id.to_hex());
        let request = Request::builder()
            .uri(finish_url)
            .method("POST")
            .header("Authorization", format!("Bearer {}", user.token))
            .body(body::Empty::new())
            .unwrap();

        // Act
        let response = ctx.app.oneshot(request).await.unwrap();

        // Assert
        assert_eq!(response.status(), StatusCode::OK, "Finish quiz endpoint failed");
    }
}