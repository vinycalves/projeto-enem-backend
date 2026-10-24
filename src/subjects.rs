use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct SubjectDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubjectsResponse {
    pub subjects: Vec<SubjectDto>,
}

#[utoipa::path(
    get,
    path = "/api/v1/subjects",
    responses((status = 200, body = SubjectsResponse))
)]
pub async fn list_subjects(
    State(state): State<AppState>,
) -> Result<Json<SubjectsResponse>, AppError> {
    let subjects = sqlx::query_as::<_, SubjectDto>(
        r#"
        SELECT id, code, name, description
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

    Ok(Json(SubjectsResponse { subjects }))
}
