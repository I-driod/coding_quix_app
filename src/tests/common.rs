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
    services::{
        phone_verify::TwilioClient, question_service::QuestionService, quiz_service::QuizService,
        user_service::UserService,
    },
    utils::{generate_jwt, hash_password},
};

pub struct TestServices {
    pub user_service: UserService,
    pub quiz_service: QuizService,
    pub question_service: QuestionService,
}
pub struct TestContext {
    pub app: Router,
    pub db: Arc<Database>,
    pub services: TestServices,
}

pub struct TestUser {
    pub id: ObjectId,
    pub token: String,
    pub role: Role,
}
pub async fn setup() -> TestContext {
    dotenv::dotenv().ok();
    unsafe {
        std::env::set_var("JWT_SECRET", std::env::var("JWT_SECRET").unwrap_or("test_secret".to_string())); // Ensure JWT_SECRET is set
    }

    let config = Config {
        mongodb_uri: std::env::var("MONGODB_TEST_URI")
            .expect("MONGODB_TEST_URI must be set for tests"),
        jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
        port: 8001,
    };

    let db = Arc::new(init_db(&config.mongodb_uri).await);

    // Clean up database and verify
    for collection in ["users", "quizzes", "categories", "questions"] {
        let result = db.collection::<bson::Document>(collection).drop().await;
        println!("Drop {} result: {:?}", collection, result);
        if let Err(e) = result {
            panic!("Failed to drop collection {}: {}", collection, e);
        }
    }

    let twilio_client = Arc::new(TwilioClient::new_from_env());
    let config_arc = Arc::new(config);

    let user_service = UserService::new(db.clone(), twilio_client.clone());
    let quiz_service = QuizService::new(db.clone(), Arc::new(crate::services::leaderboard_service::LeaderboardService::new(db.clone())));
    let question_service = QuestionService::new(db.clone());

    let app = routes::init_routes(db.clone(), config_arc);

    TestContext {
        app,
        db,
        services: TestServices { user_service, quiz_service, question_service },
    }
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
        .insert_one(user.clone(), )
        .await
        .expect("Failed to insert test user");

    let user_id = user.id.expect("Test user should have an ID after insertion");

    // Accessing environment variable is safe here as it's for test setup and not in a multi-threaded context where it could cause issues.
    let jwt_secret = unsafe {
        std::env::var("JWT_SECRET").expect("JWT_SECRET must be set")
    };
    let token = generate_jwt(&user_id.to_hex(), &role.to_string(), &jwt_secret);

    TestUser { id: user_id, token, role }
}
/// Helper to create a JSON body for requests.
pub fn json_body(json: &Value) -> Body {
    Body::from(json.to_string())
}