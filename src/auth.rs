//! Auth servis: JWT access/refresh tokeni, bcrypt lozinke

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::{AppError, AppResult};
use crate::models::domain::JwtClaims;

const ACCESS_TOKEN_MINUTES: i64 = 15;
const REFRESH_TOKEN_DAYS:   i64 = 30;

// ──────────────────────────────────────────────
// JWT
// ──────────────────────────────────────────────

pub fn generate_access_token(user_id: Uuid, username: &str, role: &str, secret: &str) -> AppResult<String> {
    let now = Utc::now();
    let claims = JwtClaims {
        sub:      user_id.to_string(),
        username: username.to_string(),
        role:     role.to_string(),
        exp:      (now + Duration::minutes(ACCESS_TOKEN_MINUTES)).timestamp() as u64,
        iat:      now.timestamp() as u64,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT encode: {}", e)))
}

pub fn verify_access_token(token: &str, secret: &str) -> AppResult<JwtClaims> {
    decode::<JwtClaims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
        .map(|td| td.claims)
        .map_err(|_| AppError::Unauthorized)
}

pub fn access_token_expires_in() -> u64 { (ACCESS_TOKEN_MINUTES * 60) as u64 }

// ──────────────────────────────────────────────
// Refresh token
// ──────────────────────────────────────────────

pub fn generate_refresh_token() -> String {
    use rand::Rng;
    let bytes: Vec<u8> = (0..48).map(|_| rand::thread_rng().gen()).collect();
    hex::encode(bytes)
}

pub fn hash_refresh_token(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    hex::encode(h.finalize())
}

pub fn refresh_token_expiry() -> chrono::DateTime<Utc> {
    Utc::now() + Duration::days(REFRESH_TOKEN_DAYS)
}

// ──────────────────────────────────────────────
// Lozinke
// ──────────────────────────────────────────────

pub fn hash_password(password: &str) -> AppResult<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("bcrypt hash: {}", e)))
}

pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    bcrypt::verify(password, hash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("bcrypt verify: {}", e)))
}
