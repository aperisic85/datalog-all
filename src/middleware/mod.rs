use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use sqlx::PgPool;

use crate::errors::AppError;

/// Izvlači API key iz headera.
/// CR300 šalje: HTTPHeaderResponse = "apikey: 50rl3puELRKVTR0UhtYMt7I9"
pub fn extract_api_key(req: &Request) -> Option<String> {
    // Native CR300 format: header "apikey: <key>"
    if let Some(val) = req.headers().get("apikey") {
        if let Ok(key) = val.to_str() {
            return Some(key.trim().to_string());
        }
    }
    // Standard Bearer token
    if let Some(val) = req.headers().get("authorization") {
        if let Ok(auth) = val.to_str() {
            if let Some(key) = auth.strip_prefix("Bearer ") {
                return Some(key.trim().to_string());
            }
        }
    }
    None
}

/// Middleware koji validira API key na ingest endpointima
pub async fn auth_middleware(
    State(pool): State<PgPool>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let key = extract_api_key(&req).ok_or(AppError::Unauthorized)?;

    let valid: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM api_keys WHERE key_hash = encode(sha256($1::bytea), 'hex') AND is_active = TRUE)"
    )
    .bind(key.as_str())
    .fetch_one(&pool)
    .await
    .map_err(AppError::Database)?;

    if !valid {
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(req).await)
}

/// Middleware koji validira JWT Bearer token na query/domain endpointima
/// Ubacuje JwtClaims u Extension za korištenje u handlerima
pub async fn jwt_middleware(
    axum::extract::State(jwt_secret): axum::extract::State<String>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .ok_or(AppError::Unauthorized)?;

    let claims = crate::auth::verify_access_token(&token, &jwt_secret)?;

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
