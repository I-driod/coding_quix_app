#[cfg(test)]
mod tests {
    use axum::{
        body,
        http::{self, Request, StatusCode},
    };
    use bson::oid::ObjectId;
    use serde_json::json;
    use tower::ServiceExt;

    use crate::models::{
        category::Category,
        question::{Difficulty, Question},
        user::Role,
    };

    use super::super::common::{create_test_user, json_body, setup};

    #[tokio::test]
    async fn test_finish_quiz_endpoint() {
        // Arrange
        let ctx = setup().await;
        let user = create_test_user(&ctx.db, Role::User, "quizuser", "+15550001111").await;

        // 1. Create a category and some questions for the quiz
        let category_id = ObjectId::new();
        let category = Category {
            id: Some(category_id),
            tags: vec![],
            parent_id: None,
            name: "Test Category".to_string(),
            image_url: Some("test.jpg".to_string()),
            top_user_id: None,
        };
        ctx.db
            .collection("categories")
            .insert_one(category, )
            .await
            .unwrap();

        for i in 0..5 {
            let question = Question {
                id: Some(ObjectId::new()),
                category_id,
                question: format!("Question {}?", i + 1),
                options: vec!["A".to_string(), "B".to_string()],
                correct_answer: "A".to_string(),
                difficulty: Difficulty::Beginner,
                timer: chrono::Duration::seconds(30),
                explanation: "".to_string(),
                tags: vec![],
                question_type: crate::models::question::QuestionType::MultipleChoice,
            };
            ctx.db.collection("questions").insert_one(question, ).await.unwrap();
        }

        // 2. Create a quiz directly via the service
        let start_quiz_result = ctx
            .services
            .quiz_service
            .start_quiz(user.id, category_id, Difficulty::Beginner, 5)
            .await;

        assert!(
            start_quiz_result.is_ok(),
            "Test setup failed: Could not start quiz"
        );
        let quiz = start_quiz_result.unwrap();
        let quiz_id = quiz.id.expect("Quiz must have an ID after creation");

        // 3. Call the /quiz/{id}/finish endpoint
        let finish_url = format!("/quiz/{}/finish", quiz_id.to_hex());
        let request = Request::builder()
            .uri(finish_url)
            .method("POST")
               .header(http::header::AUTHORIZATION, format!("Bearer {}", user.token))
            .body(body::Body::empty())
            .unwrap();

        // Act
        let response = ctx.app.oneshot(request).await.unwrap();

        // Assert
        assert_eq!(response.status(), StatusCode::OK, "Finish quiz endpoint failed");
    }
}