use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use enem_backend::{app, config::Config, state::AppState};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_config(pix_key: Option<&str>) -> Config {
    Config {
        database_url: "postgres://test:test@localhost/test".to_string(),
        bind_addr: "127.0.0.1:0".to_string(),
        cors_origin: "http://localhost:4200".to_string(),
        cookie_secure: false,
        session_ttl_days: 30,
        pix_key: pix_key.map(ToString::to_string),
    }
}

fn test_app(pool: PgPool, pix_key: Option<&str>) -> Router {
    app(AppState {
        pool,
        config: Arc::new(test_config(pix_key)),
    })
}

async fn request(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    cookie: Option<&str>,
) -> (StatusCode, Value, Option<String>) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }

    let body = body
        .map(|value| Body::from(value.to_string()))
        .unwrap_or_else(Body::empty);

    let response = app
        .clone()
        .oneshot(builder.body(body).expect("request inválido"))
        .await
        .expect("falha na requisição");

    let status = response.status();
    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(';').next().unwrap_or(value).to_string());

    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("falha ao ler body")
        .to_bytes();

    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("body não é JSON")
    };

    (status, body, set_cookie)
}

async fn register(app: &Router) -> (String, String) {
    let email = format!("aluno-{}@example.com", Uuid::now_v7());
    let (status, body, cookie) = request(
        app,
        Method::POST,
        "/api/v1/auth/register",
        Some(json!({ "email": email, "password": "senha-segura-123" })),
        None,
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["user"]["email"], email);
    (email, cookie.expect("cookie de sessão ausente"))
}

async fn first_question(pool: &PgPool) -> (Uuid, String) {
    sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, correct_option FROM questions WHERE code = 'MAT-FAFIM-001'",
    )
    .fetch_one(pool)
    .await
    .expect("questão de seed ausente")
}

#[sqlx::test(migrations = "./migrations")]
async fn register_login_me_and_logout(pool: PgPool) {
    let app = test_app(pool, Some("pix@example.com"));
    let (email, cookie) = register(&app).await;

    let (status, body, _) =
        request(&app, Method::GET, "/api/v1/auth/me", None, Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["email"], email);

    let (status, _, login_cookie) = request(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({ "email": email, "password": "senha-segura-123" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(login_cookie.is_some());

    let (status, _, _) = request(
        &app,
        Method::POST,
        "/api/v1/auth/logout",
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _, _) = request(&app, Method::GET, "/api/v1/auth/me", None, Some(&cookie)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn duplicate_email_and_wrong_password(pool: PgPool) {
    let app = test_app(pool, None);
    let (email, _) = register(&app).await;

    let (status, _, _) = request(
        &app,
        Method::POST,
        "/api/v1/auth/register",
        Some(json!({ "email": email, "password": "outra-senha-123" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, _, _) = request(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        Some(json!({ "email": email, "password": "senha-errada" })),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn answer_and_discussion_require_attempt(pool: PgPool) {
    let app = test_app(pool.clone(), None);
    let (_, cookie) = register(&app).await;
    let (question_id, correct_option) = first_question(&pool).await;

    let (status, body, _) = request(
        &app,
        Method::GET,
        &format!("/api/v1/questions/{question_id}"),
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["question"]["options"].as_array().unwrap().len(), 5);
    assert_eq!(body["question"]["has_attempted"], false);
    assert!(body["question"].get("correct_option").is_none());

    let (status, _, _) = request(
        &app,
        Method::GET,
        &format!("/api/v1/questions/{question_id}/answer"),
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, _, _) = request(
        &app,
        Method::GET,
        &format!("/api/v1/questions/{question_id}/discussion"),
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    let (status, body, _) = request(
        &app,
        Method::POST,
        &format!("/api/v1/questions/{question_id}/attempts"),
        Some(json!({ "selected_option": correct_option })),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["attempt"]["is_correct"], true);
    assert!(body["attempt"].get("correct_option").is_none());

    let (status, body, _) = request(
        &app,
        Method::GET,
        &format!("/api/v1/questions/{question_id}/answer"),
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["answer"]["correct_option"], correct_option.trim());

    let (status, body, _) = request(
        &app,
        Method::GET,
        &format!("/api/v1/questions/{question_id}/discussion"),
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["comments"].as_array().unwrap().len(), 0);

    let (status, body, _) = request(
        &app,
        Method::POST,
        &format!("/api/v1/questions/{question_id}/comments"),
        Some(json!({
            "body": "Eu escolhi essa alternativa porque isolei o x antes de substituir.",
            "option_label": correct_option
        })),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["comment"]["is_own"], true);

    let (status, body, _) = request(
        &app,
        Method::GET,
        &format!("/api/v1/questions/{question_id}/discussion"),
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["comments"].as_array().unwrap().len(), 1);

    let (status, body, _) =
        request(&app, Method::GET, "/api/v1/me/stats", None, Some(&cookie)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["stats"]["total_attempts"], 1);
    assert_eq!(body["stats"]["correct_attempts"], 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn wrong_attempt_does_not_reveal_answer(pool: PgPool) {
    let app = test_app(pool.clone(), None);
    let (_, cookie) = register(&app).await;
    let (question_id, correct_option) = first_question(&pool).await;
    let wrong_option = if correct_option.trim() == "A" {
        "B"
    } else {
        "A"
    };

    let (status, body, _) = request(
        &app,
        Method::POST,
        &format!("/api/v1/questions/{question_id}/attempts"),
        Some(json!({ "selected_option": wrong_option })),
        Some(&cookie),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["attempt"]["is_correct"], false);
    assert!(body["attempt"].get("correct_option").is_none());

    let (status, body, _) = request(
        &app,
        Method::GET,
        &format!("/api/v1/questions/{question_id}/answer"),
        None,
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["answer"]["correct_option"], correct_option.trim());
}

#[sqlx::test(migrations = "./migrations")]
async fn comment_validation_and_report(pool: PgPool) {
    let app = test_app(pool.clone(), None);
    let (_, cookie) = register(&app).await;
    let (question_id, correct_option) = first_question(&pool).await;

    request(
        &app,
        Method::POST,
        &format!("/api/v1/questions/{question_id}/attempts"),
        Some(json!({ "selected_option": correct_option })),
        Some(&cookie),
    )
    .await;

    let (status, _, _) = request(
        &app,
        Method::POST,
        &format!("/api/v1/questions/{question_id}/comments"),
        Some(json!({ "body": "   " })),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, body, _) = request(
        &app,
        Method::POST,
        &format!("/api/v1/questions/{question_id}/comments"),
        Some(json!({ "body": "Comentário válido para o teste." })),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let comment_id = body["comment"]["id"].as_str().unwrap();

    let (status, _, _) = request(
        &app,
        Method::POST,
        &format!("/api/v1/comments/{comment_id}/report"),
        Some(json!({ "reason": "Comentário de teste" })),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _, _) = request(
        &app,
        Method::POST,
        &format!("/api/v1/comments/{comment_id}/report"),
        Some(json!({ "reason": "Duplicado" })),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = "./migrations")]
async fn calendar_contains_confirmed_exam_dates(pool: PgPool) {
    let app = test_app(pool, None);
    let (status, body, _) = request(&app, Method::GET, "/api/v1/enem/calendar", None, None).await;

    assert_eq!(status, StatusCode::OK);
    let events = body["events"].as_array().unwrap();
    let first_exam = events
        .iter()
        .find(|event| event["title"] == "Provas — 1º dia")
        .expect("evento da prova ausente");
    assert_eq!(first_exam["starts_on"], "2026-11-08");
    assert_eq!(first_exam["confirmed"], true);

    let result = events
        .iter()
        .find(|event| event["title"] == "Resultado individual")
        .expect("evento de resultado ausente");
    assert_eq!(result["confirmed"], false);
    assert!(result["starts_on"].is_null());
}

#[sqlx::test(migrations = "./migrations")]
async fn pix_endpoint_respects_configuration(pool: PgPool) {
    let app = test_app(pool.clone(), Some("vinycius@example.com"));
    let (status, body, _) = request(&app, Method::GET, "/api/v1/support/pix", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["key"], "vinycius@example.com");

    let app_without_pix = test_app(pool, None);
    let (status, _, _) = request(
        &app_without_pix,
        Method::GET,
        "/api/v1/support/pix",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn curriculum_is_seeded(pool: PgPool) {
    let app = test_app(pool, None);
    let (status, body, _) =
        request(&app, Method::GET, "/api/v1/curriculum/areas", None, None).await;

    assert_eq!(status, StatusCode::OK);
    let areas = body["areas"].as_array().unwrap();
    assert_eq!(areas.len(), 4);
    assert!(
        areas
            .iter()
            .all(|area| !area["subjects"].as_array().unwrap().is_empty())
    );

    let math = areas
        .iter()
        .find(|area| area["code"] == "matematica")
        .expect("área de matemática ausente");
    assert_eq!(math["subjects"][0]["concept_count"], 3);
}
