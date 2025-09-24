use axum::Router;
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;
use utoipa::{openapi::{security::{HttpAuthScheme, HttpBuilder, SecurityScheme}, SecurityRequirement}, Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

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
        routes::admin::top_user_for_category,
        routes::admin::get_categories_with_top_users,
        routes::quiz::submit_answer,
        routes::quiz::finish_quiz
     

    ),
    components(
        schemas(
            routes::auth::RegisterRequest,
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
            models::category::CategoryWithTopUserResponse,
            models::question::CreateQuestionRequest,
            routes::quiz::SubmitAnswerRequest,

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

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let config = config::Config::from_env();
    let db = Arc::new(db::init_db(&config.mongodb_uri).await);

    let app = Router::new()
       .merge(routes::init_routes(db.clone(), Arc::new(config.clone())))
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    println!("🚀 Server running at http://{}", addr); // helpful log
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    // serve the router with the listener (axum::serve requires tokio + http1/http2 features)
    axum::serve(listener, app).await.unwrap();
     
}
