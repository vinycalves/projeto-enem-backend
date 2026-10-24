use argon2::{
    Argon2,
    password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
        rand_core::OsRng as PasswordOsRng,
    },
};
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use rand::{RngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    config::Config,
    error::AppError,
    state::AppState,
    validation::{display_name_from_email, normalize_email, validate_email, validate_password},
};

const SESSION_COOKIE: &str = "enem_session";

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserDto,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = session_token_from_headers(&parts.headers).ok_or(AppError::Unauthorized)?;
        let token_hash = hash_token(&token);

        let user = sqlx::query_as::<_, (Uuid, String, String)>(
            r#"
            SELECT u.id, u.email, u.display_name
            FROM sessions s
            JOIN users u ON u.id = s.user_id
            WHERE s.token_hash = $1 AND s.expires_at > now()
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&state.pool)
        .await?;

        let (id, email, display_name) = user.ok_or(AppError::Unauthorized)?;

        Ok(Self {
            id,
            email,
            display_name,
        })
    }
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<Credentials>,
) -> Result<Response, AppError> {
    let email = normalize_email(&payload.email);
    validate_email(&email)?;
    validate_password(&payload.password)?;

    let password_hash = hash_password(&payload.password)?;
    let user_id = Uuid::now_v7();
    let display_name = display_name_from_email(&email);

    let result = sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, display_name)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(&email)
    .bind(&password_hash)
    .bind(&display_name)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {}
        Err(sqlx::Error::Database(database_error)) if database_error.is_unique_violation() => {
            return Err(AppError::Conflict(
                "Já existe uma conta com este e-mail.".to_string(),
            ));
        }
        Err(error) => return Err(error.into()),
    }

    let (_, cookie) = create_session(&state, user_id).await?;

    let mut headers = HeaderMap::new();
    headers.append(header::SET_COOKIE, cookie);

    Ok((
        StatusCode::CREATED,
        headers,
        Json(AuthResponse {
            user: UserDto {
                id: user_id,
                email,
                display_name,
            },
        }),
    )
        .into_response())
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<Credentials>,
) -> Result<Response, AppError> {
    let email = normalize_email(&payload.email);
    validate_email(&email)?;

    let row = sqlx::query_as::<_, (Uuid, String, String, String)>(
        r#"
        SELECT id, email, display_name, password_hash
        FROM users
        WHERE lower(email) = $1
        "#,
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await?;

    let Some((user_id, user_email, display_name, password_hash)) = row else {
        return Err(AppError::Unauthorized);
    };

    if !verify_password(&payload.password, &password_hash) {
        return Err(AppError::Unauthorized);
    }

    let (_, cookie) = create_session(&state, user_id).await?;

    let mut headers = HeaderMap::new();
    headers.append(header::SET_COOKIE, cookie);

    Ok((
        StatusCode::OK,
        headers,
        Json(AuthResponse {
            user: UserDto {
                id: user_id,
                email: user_email,
                display_name,
            },
        }),
    )
        .into_response())
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    if let Some(token) = session_token_from_headers(&headers) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
            .bind(hash_token(&token))
            .execute(&state.pool)
            .await?;
    }

    let mut response_headers = HeaderMap::new();
    response_headers.append(header::SET_COOKIE, clear_session_cookie());

    Ok((StatusCode::NO_CONTENT, response_headers).into_response())
}

pub async fn me(auth: AuthUser) -> Json<AuthResponse> {
    Json(AuthResponse {
        user: UserDto {
            id: auth.id,
            email: auth.email,
            display_name: auth.display_name,
        },
    })
}

async fn create_session(
    state: &AppState,
    user_id: Uuid,
) -> Result<(String, HeaderValue), AppError> {
    let token = generate_token();
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + Duration::days(state.config.session_ttl_days);

    sqlx::query(
        r#"
        INSERT INTO sessions (id, user_id, token_hash, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    let cookie = session_cookie(&token, &state.config)?;
    Ok((token, cookie))
}

fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut PasswordOsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| {
            tracing::error!(%error, "erro ao gerar hash de senha");
            AppError::Internal
        })
}

fn verify_password(password: &str, password_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return false;
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn session_token_from_headers(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;

    cookie_header.split(';').find_map(|part| {
        let (name, value) = part.trim().split_once('=')?;
        if name == SESSION_COOKIE && !value.is_empty() {
            Some(value.to_string())
        } else {
            None
        }
    })
}

fn session_cookie(token: &str, config: &Config) -> Result<HeaderValue, AppError> {
    let max_age = config.session_ttl_days * 24 * 60 * 60;
    let secure = if config.cookie_secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}"
    ))
    .map_err(|_| AppError::Internal)
}

fn clear_session_cookie() -> HeaderValue {
    HeaderValue::from_static("enem_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_and_verifies_password() {
        let hash = hash_password("senha-segura-123").unwrap();
        assert_ne!(hash, "senha-segura-123");
        assert!(verify_password("senha-segura-123", &hash));
        assert!(!verify_password("senha-errada", &hash));
    }

    #[test]
    fn generates_unique_tokens() {
        assert_ne!(generate_token(), generate_token());
    }

    #[test]
    fn reads_session_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("outro=1; enem_session=abc123; tema=claro"),
        );
        assert_eq!(
            session_token_from_headers(&headers).as_deref(),
            Some("abc123")
        );
    }
}
