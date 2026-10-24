pub mod auth;
pub mod calendar;
pub mod chemistry;
pub mod config;
pub mod curriculum;
pub mod error;
pub mod exams;
pub mod geo;
pub mod health;
pub mod importer;
pub mod molecules;
pub mod openapi;
pub mod questions;
pub mod state;
pub mod subjects;
pub mod support;
pub mod topics;
pub mod validation;
pub mod visualizations;

use axum::{
    Json, Router,
    http::{HeaderValue, Method, header},
    routing::{get, post},
};
use tower_http::{
    compression::CompressionLayer, cors::CorsLayer, set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

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

    let public_routes = Router::new()
        .route("/api/v1/subjects", get(subjects::list_subjects))
        .route(
            "/api/v1/visualizations",
            get(visualizations::list_visualizations),
        )
        .route(
            "/api/v1/visualizations/{slug}",
            get(visualizations::get_visualization),
        )
        .route("/api/v1/enem/topics", get(topics::list_topics))
        .route(
            "/api/v1/chemistry/reactions/{id}",
            get(chemistry::get_reaction),
        )
        .route(
            "/api/v1/chemistry/datasets/{name}",
            get(chemistry::get_dataset),
        )
        .route("/api/v1/molecules/{slug}", get(molecules::get_molecule))
        .route("/api/v1/geo/layers/{layer}", get(geo::get_geo_layer))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=60"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::ETAG,
            HeaderValue::from_static("W/\"visualizations-v1\""),
        ));

    Router::new()
        .route("/api/v1/health", get(health::health))
        .route("/api-docs/openapi.json", get(openapi_json))
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
        .route("/api/v1/exams", get(exams::list_exams))
        .route("/api/v1/enem/calendar", get(calendar::list_events))
        .route("/api/v1/support/pix", get(support::pix))
        .merge(public_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .with_state(state)
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi::openapi())
}
