use axum::{Json, extract::State};
use serde::Serialize;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExamDto {
    pub id: Uuid,
    pub title: String,
    pub year: i32,
    pub area_code: String,
    pub area_name: String,
    pub area_color: String,
    pub question_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ExamsResponse {
    pub exams: Vec<ExamDto>,
}

pub async fn list_exams(State(state): State<AppState>) -> Result<Json<ExamsResponse>, AppError> {
    let exams = sqlx::query_as::<_, ExamDto>(
        r#"
        SELECT
            c.id,
            c.name AS title,
            c.sort_order AS year,
            a.code AS area_code,
            a.name AS area_name,
            a.color AS area_color,
            (SELECT count(*) FROM questions q WHERE q.concept_id = c.id) AS question_count
        FROM concepts c
        JOIN subjects s ON s.id = c.subject_id
        JOIN areas a ON a.id = s.area_id
        WHERE s.code LIKE 'oficial-%'
        ORDER BY a.sort_order, c.sort_order DESC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ExamsResponse { exams }))
}
