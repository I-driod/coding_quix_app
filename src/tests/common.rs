use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    Router,
};
use bson::oid::ObjectId;
use mongodb::Database;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

use crate::{
    config::Config,
    db::init_db,
    models::user::{Profile, Role, User},
    routes,
    services::user_service::UserService,
    utils::{generate_jwt, hash_password},
};

pub struct TestContext {
    pub app: Router,
    pub db: Arc<Database>,
}

pub struct TestUser {
    pub id: ObjectId,
    pub token: String,
    pub role: Role,
}

/// Sets up a test application instance with a clean database.
pub async fn setup() -> TestContext {
    dotenv::dotenv().ok();

    let config = Config {
        mongodb_uri: std::env::var("MONGODB_TEST_URI")
            .expect("MONGODB_TEST_URI must be set for tests"),
        jwt_secret: "test_secret".to_string(),
        port: 8001, // Use a different port for tests
    };

    let db = Arc::new(init_db(&config.mongodb_uri).await);

    // Clean up database before each test
    db.collection::<User>("users").drop(None).await.ok();
    db.collection::<crate::models::quiz::Quiz>("quizzes")
        .drop(None)
        .await
        .ok();
    db.collection::<crate::models::category::Category>("categories")
        .drop(None)
        .await
        .ok();
    db.collection::<crate::models::question::Question>("questions")
        .drop(None)
        .await
        .ok();

    let app = routes::init_routes(db.clone(), Arc::new(config));

    TestContext { app, db }
}

/// Creates a user in the database and returns the user's ID and a valid JWT.
pub async fn create_test_user(
    db: &Arc<Database>,
    role: Role,
    username: &str,
    phone: &str,
) -> TestUser {
    let user = User {
        id: Some(ObjectId::new()),
        phone_number: phone.to_string(),
        username: username.to_string(),
        password_hash: hash_password("password123"),
        role: role.clone(),
        profile: Profile {
            avatar: None,
            bio: None,
            preferred_language: None,
            country: None,
        },
        xp: 0,
        quiz_history: vec![],
    };

    db.collection("users")
        .insert_one(user.clone(), None)
        .await
        .expect("Failed to insert test user");

    let user_id = user.id.unwrap();
    let token = generate_jwt(&user_id.to_hex(), &role.to_string(), "test_secret");

    TestUser { id: user_id, token, role }
}

/// Helper to create a JSON body for requests.
pub fn json_body(json: &Value) -> Body {
    Body::from(json.to_string())
}