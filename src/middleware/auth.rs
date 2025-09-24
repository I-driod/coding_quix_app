use axum::{
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::config::Config;
use crate::utils::validate_jwt; // adjust path if Claims lives elsewhere

/// Middleware compatible with `middleware::from_fn`.
/// - Reads `Authorization: Bearer <token>` header (plain string parsing).
/// - Tries to read JWT secret from request extensions (Arc<Config>) or falls back to env var `JWT_SECRET`.
/// - Validates token, inserts `Arc<Claims>` into request extensions on success.
pub async fn auth_middleware(
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1) Get header string
    let auth_header = match req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // 2) Parse `Bearer <token>`
    let token = match auth_header.strip_prefix("Bearer ") {
        Some(t) => t,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // 3) Obtain jwt_secret: prefer Config in request extensions, otherwise fallback to env
    let jwt_secret = if let Some(cfg) = req.extensions().get::<Arc<Config>>() {
        cfg.jwt_secret.clone()
    } else if let Ok(s) = std::env::var("JWT_SECRET") {
        s
    } else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    // 4) Validate and insert claims (store Arc<Claims> to avoid Clone bounds)
    match validate_jwt(token, &jwt_secret) {
        Some(claims) => {
            req.extensions_mut().insert(Arc::new(claims));
            Ok(next.run(req).await)
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
