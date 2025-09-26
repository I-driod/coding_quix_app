use aws_config::{BehaviorVersion, Region};
use axum::Router;
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;
use utoipa::{openapi::{security::{HttpAuthScheme, HttpBuilder, SecurityScheme}, SecurityRequirement}, Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;
use shuttle_aws_rds;

use crate::{config::Config, routes::init_routes, services::phone_verify::TwilioClient};

mod config;
mod db;
mod routes;
mod services;


mod models;
mod utils;
mod middleware;





#[derive(OpenApi)]
#[openapi(
    paths(
        routes::health_check,
        routes::auth::confirm_register,
        routes::auth::start_verification,
        routes::auth::login,
        routes::user::get_user,
        routes::user::update_profile,
        routes::admin::create_category,
        routes::admin::create_question,
        routes::quiz::start_quiz,
        routes::quiz::submit_answer,
        routes::quiz::finish_quiz,
        routes::quiz::get_leaderboard,
        routes::quiz::pause_quiz,
        routes::admin::get_category,
        routes::admin::list_categories,
        routes::admin::delete_category,
        routes::admin::get_question,
        routes::admin::list_questions,
        routes::admin::delete_question

    ),
    components(
        schemas(
            routes::auth::ConfirmRegisterRequest,
            routes::auth::StartVerificationRequest,
            routes::auth::LoginRequest,
            services::user_service::LoginResponse,
            // models::user::User,
            models::user::UserResponse,
            models::user::Role,
            models::user::Profile,
            routes::user::UpdateProfileRequest,
            utils::Claims,
            models::category::CreateCategoryMultipart,
            models::question::CreateQuestionRequest,
            routes::quiz::SubmitAnswerRequest,
            routes::quiz::PauseQuizRequest,
            models::leaderboard::LeaderboardEntry,
            models::category::CategoryResponse,
            models::question::QuestionResponse,

            // routes::quiz::FiFn,
            

        
        ),
      
    ),
   modifiers(&SecurityAddon)
)]
 struct ApiDoc;

 /// runtime modifier that injects a `bearer_auth` SecurityScheme and a global SecurityRequirement
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // ensure components exists
        let comps = openapi.components.get_or_insert_with(Default::default);

        // Add a bearer SecurityScheme named "bearer_auth"
        comps.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );

     // Correct turbofish order: <NameType, ScopesIteratorType, ScopeItemType>
        // NameType = String
        // ScopesIteratorType = Vec<String>  (Vec implements IntoIterator<Item = String>)
        // ScopeItemType = String
        let sr = SecurityRequirement::new::<String, Vec<String>, String>(
            "bearer_auth".to_string(),
            Vec::<String>::new(),
        );

        // Make the scheme a global security requirement (so ops show padlocks)
        openapi.security = Some(vec![sr]);

    }
}

#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secrets: shuttle_runtime::SecretStore,
) -> shuttle_axum::ShuttleAxum {
    dotenv::dotenv().ok();

    let access_key_id = secrets
        .get("AWS_ACCESS_KEY_ID")
        .expect("Missing AWS_ACCESS_KEY_ID in Secrets.toml");
    let secret_access_key = secrets
        .get("AWS_SECRET_ACCESS_KEY")
        .expect("Missing AWS_SECRET_ACCESS_KEY in Secrets.toml");


let creds = aws_credential_types::Credentials::new(
    access_key_id,
    secret_access_key,
    None,                 // session token
    None,                 // expiry
    "loaded-from-secrets" // provider name
);
   let cfg = aws_config::defaults(BehaviorVersion::latest())
    .region(Region::new("eu-north-1"))
    .credentials_provider(creds)
    .load()
    .await;


    

let bucket_name = secrets
    .get("AWS_BUCKET_NAME")
    .expect("Missing AWS_BUCKET_NAME in Secrets.toml")
    .to_string();



    let s3_client = aws_sdk_s3::Client::new(&cfg);

    // Then wrap or pass s3_client in your state
let s3_service = Arc::new(services::s3_service::S3Service::new(
    s3_client,
    bucket_name.clone(),
));

    // let config = config::Config::from_env();

    let config = Config::from_secrets(&secrets);
    let db = Arc::new(db::init_db(&config.mongodb_uri).await);

      let twilio = Arc::new(TwilioClient::new_from_secrets(&secrets));

    let app = Router::new()
          .merge(init_routes(db, Arc::new(config), s3_service, twilio))
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .layer(CorsLayer::permissive());

    Ok(app.into())
}

#[cfg(test)]
mod tests;
