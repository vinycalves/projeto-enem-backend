use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct VisualizationParam {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    #[schema(value_type = Object)]
    pub default: serde_json::Value,
    pub unit: Option<String>,
    pub options: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct VisualizationSummary {
    pub id: Uuid,
    pub slug: String,
    pub subject_code: String,
    pub subject_name: String,
    pub dimension: String,
    pub title: String,
    pub summary: String,
    pub difficulty: String,
    pub model_kind: String,
    pub tier: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VisualizationQuizQuestion {
    pub question: String,
    pub options: Vec<String>,
    pub correct_index: usize,
    pub explanation: String,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct VisualizationTopicDto {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VisualizationDetail {
    pub id: Uuid,
    pub slug: String,
    pub subject_code: String,
    pub subject_name: String,
    pub dimension: String,
    pub title: String,
    pub summary: String,
    pub learning_objectives: Vec<String>,
    pub difficulty: String,
    pub params: Vec<VisualizationParam>,
    pub common_mistakes: Vec<String>,
    pub sources: Vec<String>,
    pub component_key: String,
    pub model_kind: String,
    pub dimension_rationale: String,
    pub tier: String,
    pub simplifications: Vec<String>,
    pub enem_notes: Vec<String>,
    pub quiz: Vec<VisualizationQuizQuestion>,
    pub topics: Vec<VisualizationTopicDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VisualizationListResponse {
    pub items: Vec<VisualizationSummary>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VisualizationResponse {
    pub visualization: VisualizationDetail,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct VisualizationQuery {
    pub subject: Option<String>,
    pub dimension: Option<String>,
    pub topic: Option<String>,
    pub difficulty: Option<String>,
    pub q: Option<String>,
    #[param(default = 1, minimum = 1)]
    pub page: Option<i64>,
    #[param(default = 12, minimum = 1, maximum = 50)]
    pub per_page: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct VisualizationDetailRow {
    id: Uuid,
    slug: String,
    subject_code: String,
    subject_name: String,
    dimension: String,
    title: String,
    summary: String,
    learning_objectives: serde_json::Value,
    difficulty: String,
    params: serde_json::Value,
    common_mistakes: serde_json::Value,
    sources: serde_json::Value,
    component_key: String,
    model_kind: String,
    dimension_rationale: String,
    tier: String,
    simplifications: serde_json::Value,
    enem_notes: serde_json::Value,
    quiz: serde_json::Value,
}

#[utoipa::path(
    get,
    path = "/api/v1/visualizations",
    params(VisualizationQuery),
    responses((status = 200, body = VisualizationListResponse))
)]
pub async fn list_visualizations(
    State(state): State<AppState>,
    Query(query): Query<VisualizationQuery>,
) -> Result<Json<VisualizationListResponse>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(12).clamp(1, 50);
    let offset = (page - 1) * per_page;
    let search = query.q.as_ref().map(|value| format!("%{}%", value.trim()));

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM visualizations v
        WHERE v.published = true
          AND ($1::text IS NULL OR v.subject_code = $1)
          AND ($2::text IS NULL OR v.dimension = $2)
          AND ($3::text IS NULL OR EXISTS (
              SELECT 1
              FROM visualization_topics vt
              JOIN enem_topics t ON t.id = vt.topic_id
              WHERE vt.visualization_id = v.id AND t.slug = $3
          ))
          AND ($4::text IS NULL OR v.difficulty = $4)
          AND ($5::text IS NULL OR v.title ILIKE $5 OR v.summary ILIKE $5)
        "#,
    )
    .bind(&query.subject)
    .bind(&query.dimension)
    .bind(&query.topic)
    .bind(&query.difficulty)
    .bind(&search)
    .fetch_one(&state.pool)
    .await?;

    let items = sqlx::query_as::<_, VisualizationSummary>(
        r#"
        SELECT
            v.id,
            v.slug,
            v.subject_code,
            s.name AS subject_name,
            v.dimension,
            v.title,
            v.summary,
            v.difficulty,
            v.model_kind,
            v.tier
        FROM visualizations v
        JOIN subjects s ON s.code = v.subject_code
        WHERE v.published = true
          AND ($1::text IS NULL OR v.subject_code = $1)
          AND ($2::text IS NULL OR v.dimension = $2)
          AND ($3::text IS NULL OR EXISTS (
              SELECT 1
              FROM visualization_topics vt
              JOIN enem_topics t ON t.id = vt.topic_id
              WHERE vt.visualization_id = v.id AND t.slug = $3
          ))
          AND ($4::text IS NULL OR v.difficulty = $4)
          AND ($5::text IS NULL OR v.title ILIKE $5 OR v.summary ILIKE $5)
        ORDER BY v.subject_code, v.dimension, v.title
        LIMIT $6 OFFSET $7
        "#,
    )
    .bind(&query.subject)
    .bind(&query.dimension)
    .bind(&query.topic)
    .bind(&query.difficulty)
    .bind(&search)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(VisualizationListResponse {
        items,
        total,
        page,
        per_page,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/visualizations/{slug}",
    params(("slug" = String, Path, description = "Slug único da visualização")),
    responses(
        (status = 200, body = VisualizationResponse),
        (status = 404, description = "Visualização não encontrada")
    )
)]
pub async fn get_visualization(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<VisualizationResponse>, AppError> {
    let row = sqlx::query_as::<_, VisualizationDetailRow>(
        r#"
        SELECT
            v.id,
            v.slug,
            v.subject_code,
            s.name AS subject_name,
            v.dimension,
            v.title,
            v.summary,
            v.learning_objectives,
            v.difficulty,
            v.params,
            v.common_mistakes,
            v.sources,
            v.component_key,
            v.model_kind,
            v.dimension_rationale,
            v.tier,
            v.simplifications,
            v.enem_notes,
            v.quiz
        FROM visualizations v
        JOIN subjects s ON s.code = v.subject_code
        WHERE v.slug = $1 AND v.published = true
        "#,
    )
    .bind(&slug)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Visualização não encontrada.".to_string()))?;

    let topics = sqlx::query_as::<_, VisualizationTopicDto>(
        r#"
        SELECT t.id, t.slug, t.name
        FROM enem_topics t
        JOIN visualization_topics vt ON vt.topic_id = t.id
        WHERE vt.visualization_id = $1
        ORDER BY t.sort_order, t.name
        "#,
    )
    .bind(row.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(VisualizationResponse {
        visualization: VisualizationDetail {
            id: row.id,
            slug: row.slug,
            subject_code: row.subject_code,
            subject_name: row.subject_name,
            dimension: row.dimension,
            title: row.title,
            summary: row.summary,
            learning_objectives: parse_json(row.learning_objectives)?,
            difficulty: row.difficulty,
            params: parse_json(row.params)?,
            common_mistakes: parse_json(row.common_mistakes)?,
            sources: parse_json(row.sources)?,
            component_key: row.component_key,
            model_kind: row.model_kind,
            dimension_rationale: row.dimension_rationale,
            tier: row.tier,
            simplifications: parse_json(row.simplifications)?,
            enem_notes: parse_json(row.enem_notes)?,
            quiz: parse_json(row.quiz)?,
            topics,
        },
    }))
}

fn parse_json<T: DeserializeOwned>(value: serde_json::Value) -> Result<T, AppError> {
    serde_json::from_value(value).map_err(|error| {
        tracing::error!(%error, "JSONB inválido na visualização");
        AppError::Internal
    })
}
