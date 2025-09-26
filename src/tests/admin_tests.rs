#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        http::{Request, StatusCode},
    };
    use bson::oid::ObjectId;
    use tower::ServiceExt;
    use std::fs;

    use crate::{
        models::{
            category::{Category, CreateCategoryResponse},
            question::{Difficulty, Question, QuestionType},
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
        // assert_eq!(response.status(), StatusCode::OK);
        // let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        // let categories: Vec<CategoryWithTopUserResponse> = serde_json::from_slice(&body).unwrap();
        // assert_eq!(categories.len(), 1);
        // let cat_response = &categories[0];
        // assert_eq!(cat_response.category.name, "Test Category");
        // assert!(cat_response.top_user.is_some());
        // assert_eq!(cat_response.top_user.as_ref().unwrap().username, "topgun");

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

    #[tokio::test]
    async fn test_create_category() {
        // Arrange
        let ctx = setup().await;
        let admin_user = create_test_user(&ctx.db, Role::Admin, "admin_cat", "+15559998889").await;

        let category_name = "New Test Category".to_string();
        let tags = vec!["tag1".to_string(), "tag2".to_string()];
        let image_bytes = b"fake image content"; // Simulate image content
        let image_filename = "test_image.png";

        let boundary = "--------------------------boundary";
        let body_str = format!(
            "--{}\nContent-Disposition: form-data; name=\"name\"\n\r\n{}\n--{}\nContent-Disposition: form-data; name=\"tags\"\n\r\n{}\n--{}\nContent-Disposition: form-data; name=\"image\"; filename=\"{}\"\r\nContent-Type: image/png\r\n\r\n{}\n--{}--\r\n",
            boundary,
            category_name,
            boundary,
            tags.join(","),
            boundary,
            image_filename,
            std::str::from_utf8(image_bytes).unwrap(),
            boundary
        );

        println!("Request body: {}", body_str);

        let request = Request::builder()
            .method("POST")
            .uri("/admin/categories")
            .header("Authorization", format!("Bearer {}", admin_user.token))
            .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
            .header("Content-Length", body_str.len())
            .body(axum::body::Body::from(body_str.into_bytes()))
            .unwrap();

        let response = ctx.app.clone().oneshot(request).await.unwrap();

        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

        println!("Response status: {}", status);
        println!("Response body: {:?}", String::from_utf8_lossy(&body));

        assert_eq!(status, StatusCode::CREATED);

        let create_category_response: CreateCategoryResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(create_category_response.message, "Category created successfully");
        assert_eq!(create_category_response.category.name, category_name);
        assert_eq!(create_category_response.category.tags, tags);
        assert!(create_category_response.category.image_url.is_some());

        // Clean up the uploaded file.
        let uploaded_filename = create_category_response.category.image_url.unwrap().split('/').last().unwrap().to_string();
        let filepath = format!("uploads/{}", uploaded_filename);
        fs::remove_file(&filepath).unwrap();
    }

    #[tokio::test]
    async fn test_create_question() {
        // Arrange
        let ctx = setup().await;
        let admin_user = create_test_user(&ctx.db, Role::Admin, "admin_q", "+15559998880").await;

        // Create a category first
        let category_id = ObjectId::new();
        let category = Category {
            id: Some(category_id),
            name: "Question Category".to_string(),
            image_url: None,
            top_user_id: None,
            tags: vec![],
            parent_id: None,
        };
        ctx.db.collection("categories").insert_one(category, ).await.unwrap();

        let question_data = crate::models::question::Question {
            id: None,
            category_id,
            question: "What is the capital of France?".to_string(),
            options: vec!["Paris".to_string(), "London".to_string(), "Berlin".to_string()],
            correct_answer: "Paris".to_string(),
            difficulty: Difficulty::Beginner,
            timer: chrono::Duration::seconds(30),
            tags: vec!["geography".to_string()],
            explanation: "Paris is the capital and most populous city of France.".to_string(),
            question_type: QuestionType::MultipleChoice,
        };

        let request = Request::builder()
            .method("POST")
            .uri("/admin/questions")
            .header("Authorization", format!("Bearer {}", admin_user.token))
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(serde_json::to_string(&question_data).unwrap()))
            .unwrap();

        // Act
        ctx.app.clone().oneshot(request).await.unwrap();

        // Assert
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let create_question_response: crate::models::question::CreateQuestionResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(create_question_response.message, "Question created successfully");
        assert_eq!(create_question_response.question.text, question_data.question);
        assert_eq!(create_question_response.question.category_id, question_data.category_id.to_hex());
        assert_eq!(create_question_response.question.correct_answer, question_data.correct_answer);
    }
}