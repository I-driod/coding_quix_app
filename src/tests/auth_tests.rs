#[cfg(test)]
mod tests {
    use axum::{
        body::to_bytes,
        http::{self, Request, StatusCode},
    };
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use crate::{models::user::Role, services::user_service::LoginResponse};

    use super::super::common::{create_test_user, json_body, setup};

    #[tokio::test]
    async fn test_login_success() {
        // Arrange
        let ctx = setup().await;
        let phone_number = "+15551112222".to_string();
        let password = "password123".to_string();
        create_test_user(&ctx.db, Role::User, "testuser", &phone_number).await;

        let login_request = json!({
            "phone_number": phone_number,
            "password": password
        });

        // Act
        let response = ctx
            .app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/login")
                    .header(http::header::CONTENT_TYPE, "application/json")
                    .body(json_body(&login_request))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Assert
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let login_response: LoginResponse = serde_json::from_slice(&body).unwrap();
        assert!(login_response.token.is_some());
        assert_eq!(login_response.user.unwrap().username, "testuser");
    }
}