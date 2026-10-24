pub mod auth;
pub mod calendar;
pub mod config;
pub mod curriculum;
pub mod error;
pub mod questions;
pub mod state;
pub mod support;
pub mod validation;

use axum::{
    Json, Router,
    http::{HeaderValue, Method, header},
    routing::{get, post},
};
use serde_json::json;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

pub fn app(state: AppState) -> Router {
    let cors_origin = state
        .config
        .cors_origin
        .parse::<HeaderValue>()
        .expect("CORS_ORIGIN deve ser uma origem válida");

    let cors = CorsLayer::new()
        .allow_origin(cors_origin)
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/curriculum/areas", get(curriculum::list_areas))
        .route(
            "/api/v1/curriculum/subjects/{id}/concepts",
            get(curriculum::list_concepts),
        )
        .route(
            "/api/v1/curriculum/concepts/{id}",
            get(curriculum::get_concept),
        )
        .route("/api/v1/questions", get(questions::list_questions))
        .route("/api/v1/questions/{id}", get(questions::get_question))
        .route(
            "/api/v1/questions/{id}/attempts",
            post(questions::create_attempt),
        )
        .route(
            "/api/v1/questions/{id}/discussion",
            get(questions::get_discussion),
        )
        .route(
            "/api/v1/questions/{id}/comments",
            post(questions::create_comment),
        )
        .route("/api/v1/questions/{id}/answer", get(questions::get_answer))
        .route(
            "/api/v1/comments/{id}/report",
            post(questions::report_comment),
        )
        .route("/api/v1/me/stats", get(questions::my_stats))
        .route("/api/v1/enem/calendar", get(calendar::list_events))
        .route("/api/v1/support/pix", get(support::pix))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}
