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

#[sqlx::test(migrations = "./migrations")]
async fn question_assets_and_language_are_returned(pool: PgPool) {
    let app = test_app(pool.clone(), None);
    let (_, cookie) = register(&app).await;
    let concept_id: Uuid =
        sqlx::query_scalar("SELECT id FROM concepts WHERE code = 'mat.funcao-afim'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let question_id = Uuid::now_v7();

    sqlx::query(
        r#"
        INSERT INTO questions (
            id, concept_id, code, statement, option_a, option_b, option_c,
            option_d, option_e, correct_option, explanation, difficulty,
            source, license, language
        )
        VALUES ($1, $2, 'TEST-ASSET-001', 'Questão com imagem', 'A', 'B', 'C', 'D', 'E',
                'A', 'Explicação', 'medio', 'Teste', 'CC BY-SA 4.0', 'ingles')
        "#,
    )
    .bind(question_id)
    .bind(concept_id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO question_assets (id, question_id, kind, option_label, url, position)
        VALUES ($1, $2, 'alternative', 'A', 'https://example.com/a.png', 0)
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(question_id)
    .execute(&pool)
    .await
    .unwrap();

    let (status, body, _) = request(
        &app,
        Method::GET,
        &format!("/api/v1/questions/{question_id}"),
        None,
        Some(&cookie),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["question"]["language"], "ingles");
    assert_eq!(
        body["question"]["options"][0]["image_url"],
        "https://example.com/a.png"
    );
    assert_eq!(body["question"]["assets"][0]["kind"], "alternative");
}

#[sqlx::test(migrations = "./migrations")]
async fn official_exams_are_separate_from_curriculum_concepts(pool: PgPool) {
    let app = test_app(pool.clone(), None);
    let area_id: Uuid = sqlx::query_scalar("SELECT id FROM areas WHERE code = 'linguagens'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let subject_id = Uuid::now_v7();
    let concept_id = Uuid::now_v7();

    sqlx::query(
        r#"
        INSERT INTO subjects (id, area_id, code, name, description, sort_order)
        VALUES ($1, $2, 'oficial-linguagens', 'Questões oficiais — Linguagens',
                'Provas oficiais', 100)
        "#,
    )
    .bind(subject_id)
    .bind(area_id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO concepts (id, subject_id, code, name, summary, explanation, difficulty, sort_order)
        VALUES ($1, $2, 'oficial-linguagens-2023', 'ENEM 2023',
                'Prova oficial', 'Prova oficial', 'medio', 2023)
        "#,
    )
    .bind(concept_id)
    .bind(subject_id)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        INSERT INTO questions (
            id, concept_id, code, statement, option_a, option_b, option_c,
            option_d, option_e, correct_option, explanation, difficulty,
            source, license, year, language, external_id
        )
        VALUES ($1, $2, 'TEST-EXAM-001', 'Questão de prova', 'A', 'B', 'C', 'D', 'E',
                'A', 'Explicação', 'medio', 'INEP', 'Público', 2023, 'ingles',
                'test:2023:1:ingles')
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(concept_id)
    .execute(&pool)
    .await
    .unwrap();

    let (status, body, _) = request(&app, Method::GET, "/api/v1/exams", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let exams = body["exams"].as_array().unwrap();
    assert_eq!(exams.len(), 1);
    assert_eq!(exams[0]["title"], "ENEM 2023");
    assert_eq!(exams[0]["year"], 2023);
    assert_eq!(exams[0]["question_count"], 1);
    assert_eq!(exams[0]["area_code"], "linguagens");

    let (status, body, _) =
        request(&app, Method::GET, "/api/v1/curriculum/areas", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let has_official_subject = body["areas"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|area| area["subjects"].as_array().unwrap())
        .any(|subject| subject["code"] == "oficial-linguagens");
    assert!(!has_official_subject);
}

#[sqlx::test(migrations = "./migrations")]
async fn visualization_catalog_is_seeded_and_filterable(pool: PgPool) {
    let app = test_app(pool, None);

    let (status, body, _) = request(&app, Method::GET, "/api/v1/subjects", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["subjects"].as_array().unwrap().len(), 4);

    let (status, body, _) = request(&app, Method::GET, "/api/v1/visualizations", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 2);
    assert_eq!(body["items"].as_array().unwrap().len(), 2);

    let (status, body, _) = request(
        &app,
        Method::GET,
        "/api/v1/visualizations?subject=fisica&dimension=2d",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
    assert_eq!(body["items"][0]["slug"], "lancamento-obliquo");

    let (status, body, _) = request(
        &app,
        Method::GET,
        "/api/v1/visualizations/lancamento-obliquo",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["visualization"]["component_key"], "projectile-2d");
    assert_eq!(body["visualization"]["params"].as_array().unwrap().len(), 5);
    assert_eq!(
        body["visualization"]["topics"][0]["slug"],
        "lancamento-obliquo"
    );

    let (status, _, _) = request(
        &app,
        Method::GET,
        "/api/v1/visualizations/nao-existe",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "./migrations")]
async fn topics_molecules_geo_and_openapi_are_available(pool: PgPool) {
    let app = test_app(pool, None);

    let (status, body, _) = request(&app, Method::GET, "/api/v1/enem/topics", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["subjects"].as_array().unwrap().len(), 4);
    assert!(body["subjects"][0]["topics"].as_array().unwrap().len() >= 2);

    let (status, body, _) = request(&app, Method::GET, "/api/v1/molecules/agua", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["molecule"]["formula"], "H2O");
    assert_eq!(body["molecule"]["atoms"].as_array().unwrap().len(), 3);

    let (status, _, _) = request(
        &app,
        Method::GET,
        "/api/v1/geo/layers/nao-existe",
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, body, _) = request(&app, Method::GET, "/api-docs/openapi.json", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["paths"]["/api/v1/visualizations"].is_object());
    assert!(body["paths"]["/api/v1/molecules/{slug}"].is_object());
}
