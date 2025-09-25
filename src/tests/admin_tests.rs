#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        http::{Request, StatusCode},
    };
    use serde::Deserialize;
    use bson::oid::ObjectId;
    use tower::ServiceExt;

    use crate::{
        models::{
            category::Category,
            question::{Difficulty, Question},
            user::{Role, UserResponse},
        },
        tests::common::{create_test_user, setup},
    };
    
    

    #[tokio::test]
    async fn test_get_categories_with_top_users() {
        // Arrange
        let ctx = setup().await;
        let admin_user = create_test_user(&ctx.db, Role::Admin, "admin", "+15559998888").await;
        let top_user = create_test_user(&ctx.db, Role::User, "topgun", "+15551112222").await;
        let other_user = create_test_user(&ctx.db, Role::User, "secondbest", "+15553334444").await;

        // Create a category and questions
        let category_id = ObjectId::new();
        let category = Category {
            id: Some(category_id),
            name: "Test Category".to_string(),
            image_url: Some("test.jpg".to_string()),
            top_user_id: None,
            tags: vec![],
            parent_id: None,
        };
        ctx.db.collection("categories").insert_one(category, ).await.unwrap();

        for i in 0..5 {
            let question = Question {
                id: Some(ObjectId::new()),
                category_id,
                question: format!("Question {}?", i + 1),
                options: vec!["A".to_string(), "B".to_string()],
                correct_answer: "A".to_string(),
                difficulty: Difficulty::Beginner,
                timer: chrono::Duration::seconds(30),
                tags: vec![],
                explanation: "".to_string(),
                question_type: crate::models::question::QuestionType::MultipleChoice,
            };
            ctx.db.collection("questions").insert_one(question, ).await.unwrap();
        }

        // Simulate quizzes being played and finished
        let top_user_quiz = ctx.services.quiz_service.start_quiz(top_user.id, category_id, Difficulty::Beginner, 5).await.unwrap();
        let top_quiz_id = top_user_quiz.id.unwrap();
        // Submit correct answers for top_user (all "A", time_taken=10s for bonus)
        for q_id in top_user_quiz.questions.iter() {
            ctx.services.quiz_service.submit_answer(top_quiz_id, *q_id, "A".to_string(), 10).await.unwrap();
        }
        ctx.services.quiz_service.finish_quiz(&ctx.services.user_service, top_quiz_id).await.unwrap();

        let other_user_quiz = ctx.services.quiz_service.start_quiz(other_user.id, category_id, Difficulty::Beginner, 5).await.unwrap();
        let other_quiz_id = other_user_quiz.id.unwrap();
        // Submit incorrect answers for other_user (all "B")
        for q_id in other_user_quiz.questions.iter() {
            ctx.services.quiz_service.submit_answer(other_quiz_id, *q_id, "B".to_string(), 20).await.unwrap();
        }
        ctx.services.quiz_service.finish_quiz(&ctx.services.user_service, other_quiz_id).await.unwrap();

        // Act: Test /admin/categories_with_top_users
        let request = Request::builder()
            .uri("/admin/categories_with_top_users")
            .header("Authorization", format!("Bearer {}", admin_user.token))
            .body(axum::body::Body::empty())
            .unwrap();

        let response = ctx.app.clone().oneshot(request).await.unwrap();

        // Assert
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let categories: Vec<crate::models::category::CategoryWithTopUserResponse> = serde_json::from_slice(&body).unwrap();
        assert_eq!(categories.len(), 1);
        let cat_response = &categories[0];
        assert_eq!(cat_response.category.name, "Test Category");
        assert!(cat_response.top_user.is_some());
        assert_eq!(cat_response.top_user.as_ref().unwrap().username, "topgun");

        // Act: Test /admin/categories/{id}/top_user
        let top_user_url = format!("/admin/categories/{}/top_user", category_id.to_hex());
        let request = Request::builder()
            .uri(top_user_url)
            .header("Authorization", format!("Bearer {}", admin_user.token))
            .body(axum::body::Body::empty())
            .unwrap();
        
        let response = ctx.app.oneshot(request).await.unwrap();

        // Assert
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let top_user_response: UserResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(top_user_response.username, "topgun");
    }
}