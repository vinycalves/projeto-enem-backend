use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct TopicDto {
    pub id: Uuid,
    pub subject_code: String,
    pub parent_id: Option<Uuid>,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubjectTopicsDto {
    pub subject_code: String,
    pub subject_name: String,
    pub topics: Vec<TopicDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TopicsResponse {
    pub subjects: Vec<SubjectTopicsDto>,
}

#[utoipa::path(
    get,
    path = "/api/v1/enem/topics",
    responses((status = 200, body = TopicsResponse))
)]
pub async fn list_topics(State(state): State<AppState>) -> Result<Json<TopicsResponse>, AppError> {
    let subjects = sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT code, name
        FROM subjects
        WHERE code IN ('biologia', 'fisica', 'quimica', 'geografia')
        ORDER BY CASE code
            WHEN 'biologia' THEN 1
            WHEN 'fisica' THEN 2
            WHEN 'quimica' THEN 3
            WHEN 'geografia' THEN 4
            ELSE 5
        END
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::with_capacity(subjects.len());

    for (subject_code, subject_name) in subjects {
        let topics = sqlx::query_as::<_, TopicDto>(
            r#"
            SELECT id, subject_code, parent_id, slug, name, description, sort_order
            FROM enem_topics
            WHERE subject_code = $1
            ORDER BY sort_order, name
            "#,
        )
        .bind(&subject_code)
        .fetch_all(&state.pool)
        .await?;

        response.push(SubjectTopicsDto {
            subject_code,
            subject_name,
            topics,
        });
    }

    Ok(Json(TopicsResponse { subjects: response }))
}
