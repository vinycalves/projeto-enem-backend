use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize)]
pub struct SubjectDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: String,
    pub concept_count: i64,
}

#[derive(Debug, Serialize)]
pub struct AreaDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: String,
    pub color: String,
    pub icon: String,
    pub subjects: Vec<SubjectDto>,
}

#[derive(Debug, Serialize)]
pub struct AreasResponse {
    pub areas: Vec<AreaDto>,
}

#[derive(Debug, Serialize)]
pub struct SubjectHeader {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: String,
    pub area_name: String,
}

#[derive(Debug, Serialize)]
pub struct ConceptSummary {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub summary: String,
    pub difficulty: String,
    pub question_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ConceptsResponse {
    pub subject: SubjectHeader,
    pub concepts: Vec<ConceptSummary>,
}

#[derive(Debug, Serialize)]
pub struct ConceptDto {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub summary: String,
    pub explanation: String,
    pub difficulty: String,
    pub subject_name: String,
    pub area_name: String,
    pub question_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ConceptResponse {
    pub concept: ConceptDto,
}

pub async fn list_areas(State(state): State<AppState>) -> Result<Json<AreasResponse>, AppError> {
    let area_rows = sqlx::query_as::<_, (Uuid, String, String, String, String, String)>(
        r#"
        SELECT id, code, name, description, color, icon
        FROM areas
        ORDER BY sort_order, name
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let subject_rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, i64)>(
        r#"
        SELECT
            s.area_id,
            s.id,
            s.code,
            s.name,
            s.description,
            (SELECT count(*) FROM concepts c WHERE c.subject_id = s.id) AS concept_count
        FROM subjects s
        WHERE s.code NOT LIKE 'oficial-%'
        ORDER BY s.sort_order, s.name
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let mut subjects_by_area: HashMap<Uuid, Vec<SubjectDto>> = HashMap::new();
    for (area_id, id, code, name, description, concept_count) in subject_rows {
        subjects_by_area
            .entry(area_id)
            .or_default()
            .push(SubjectDto {
                id,
                code,
                name,
                description,
                concept_count,
            });
    }

    let areas = area_rows
        .into_iter()
        .map(|(id, code, name, description, color, icon)| AreaDto {
            id,
            code,
            name,
            description,
            color,
            icon,
            subjects: subjects_by_area.remove(&id).unwrap_or_default(),
        })
        .collect();

    Ok(Json(AreasResponse { areas }))
}

pub async fn list_concepts(
    State(state): State<AppState>,
    Path(subject_id): Path<Uuid>,
) -> Result<Json<ConceptsResponse>, AppError> {
    let subject = sqlx::query_as::<_, (Uuid, String, String, String, String)>(
        r#"
        SELECT s.id, s.code, s.name, s.description, a.name
        FROM subjects s
        JOIN areas a ON a.id = s.area_id
        WHERE s.id = $1
        "#,
    )
    .bind(subject_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Disciplina não encontrada.".to_string()))?;

    let concept_rows = sqlx::query_as::<_, (Uuid, String, String, String, String, i64)>(
        r#"
        SELECT
            c.id,
            c.code,
            c.name,
            c.summary,
            c.difficulty,
            (SELECT count(*) FROM questions q WHERE q.concept_id = c.id) AS question_count
        FROM concepts c
        WHERE c.subject_id = $1
        ORDER BY c.sort_order, c.name
        "#,
    )
    .bind(subject_id)
    .fetch_all(&state.pool)
    .await?;

    let concepts = concept_rows
        .into_iter()
        .map(
            |(id, code, name, summary, difficulty, question_count)| ConceptSummary {
                id,
                code,
                name,
                summary,
                difficulty,
                question_count,
            },
        )
        .collect();

    Ok(Json(ConceptsResponse {
        subject: SubjectHeader {
            id: subject.0,
            code: subject.1,
            name: subject.2,
            description: subject.3,
            area_name: subject.4,
        },
        concepts,
    }))
}

pub async fn get_concept(
    State(state): State<AppState>,
    Path(concept_id): Path<Uuid>,
) -> Result<Json<ConceptResponse>, AppError> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            i64,
        ),
    >(
        r#"
        SELECT
            c.id,
            c.code,
            c.name,
            c.summary,
            c.explanation,
            c.difficulty,
            s.name,
            a.name,
            (SELECT count(*) FROM questions q WHERE q.concept_id = c.id) AS question_count
        FROM concepts c
        JOIN subjects s ON s.id = c.subject_id
        JOIN areas a ON a.id = s.area_id
        WHERE c.id = $1
        "#,
    )
    .bind(concept_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Conceito não encontrado.".to_string()))?;

    Ok(Json(ConceptResponse {
        concept: ConceptDto {
            id: row.0,
            code: row.1,
            name: row.2,
            summary: row.3,
            explanation: row.4,
            difficulty: row.5,
            subject_name: row.6,
            area_name: row.7,
            question_count: row.8,
        },
    }))
}
