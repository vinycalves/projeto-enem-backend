use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    state::AppState,
    validation::{validate_comment_body, validate_option},
};

#[derive(Debug, Deserialize)]
pub struct ListQuestionsQuery {
    pub concept_id: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
struct QuestionListRow {
    id: Uuid,
    code: String,
    statement: String,
    difficulty: String,
    language: Option<String>,
    is_official: bool,
    exam_year: Option<i32>,
    concept_id: Uuid,
    concept_name: String,
    subject_name: String,
    area_name: String,
    source: String,
    license: String,
    has_attempted: bool,
    last_attempt_is_correct: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct QuestionListItem {
    pub id: Uuid,
    pub code: String,
    pub statement: String,
    pub difficulty: String,
    pub language: Option<String>,
    pub is_official: bool,
    pub exam_year: Option<i32>,
    pub concept_id: Uuid,
    pub concept_name: String,
    pub subject_name: String,
    pub area_name: String,
    pub source: String,
    pub license: String,
    pub has_attempted: bool,
    pub last_attempt_is_correct: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct QuestionsResponse {
    pub questions: Vec<QuestionListItem>,
}

#[derive(Debug, sqlx::FromRow)]
struct QuestionDetailRow {
    id: Uuid,
    code: String,
    statement: String,
    difficulty: String,
    language: Option<String>,
    is_official: bool,
    exam_year: Option<i32>,
    concept_id: Uuid,
    concept_name: String,
    subject_name: String,
    area_name: String,
    source: String,
    license: String,
    has_attempted: bool,
    last_attempt_is_correct: Option<bool>,
    option_a: String,
    option_b: String,
    option_c: String,
    option_d: String,
    option_e: String,
}

#[derive(Debug, Serialize)]
pub struct OptionDto {
    pub label: String,
    pub text: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct QuestionAssetDto {
    pub kind: String,
    pub option_label: Option<String>,
    pub url: String,
    pub position: i32,
}

#[derive(Debug, Serialize)]
pub struct QuestionDetail {
    pub id: Uuid,
    pub code: String,
    pub statement: String,
    pub options: Vec<OptionDto>,
    pub difficulty: String,
    pub language: Option<String>,
    pub is_official: bool,
    pub exam_year: Option<i32>,
    pub assets: Vec<QuestionAssetDto>,
    pub concept_id: Uuid,
    pub concept_name: String,
    pub subject_name: String,
    pub area_name: String,
    pub source: String,
    pub license: String,
    pub has_attempted: bool,
    pub last_attempt_is_correct: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct QuestionResponse {
    pub question: QuestionDetail,
}

#[derive(Debug, Deserialize)]
pub struct CreateAttempt {
    pub selected_option: String,
}

#[derive(Debug, Serialize)]
pub struct AttemptDto {
    pub id: Uuid,
    pub is_correct: bool,
    pub selected_option: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AttemptResponse {
    pub attempt: AttemptDto,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CommentDto {
    pub id: Uuid,
    pub author: String,
    pub option_label: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub is_own: bool,
}

#[derive(Debug, Serialize)]
pub struct DiscussionResponse {
    pub question_id: Uuid,
    pub comments: Vec<CommentDto>,
}

#[derive(Debug, Deserialize)]
pub struct CreateComment {
    pub body: String,
    pub option_label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub comment: CommentDto,
}

#[derive(Debug, Deserialize)]
pub struct ReportComment {
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct AnswerDto {
    pub correct_option: String,
    pub explanation: String,
    pub source: String,
    pub license: String,
}

#[derive(Debug, Serialize)]
pub struct AnswerResponse {
    pub answer: AnswerDto,
}

#[derive(Debug, sqlx::FromRow)]
struct StatsRow {
    total_attempts: i64,
    correct_attempts: i64,
    questions_answered: i64,
}

#[derive(Debug, Serialize)]
pub struct StatsDto {
    pub total_attempts: i64,
    pub correct_attempts: i64,
    pub questions_answered: i64,
    pub accuracy: f64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RecentAttemptDto {
    pub question_id: Uuid,
    pub question_code: String,
    pub statement_preview: String,
    pub is_correct: bool,
    pub created_at: DateTime<Utc>,
    pub concept_name: String,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub stats: StatsDto,
    pub recent_attempts: Vec<RecentAttemptDto>,
}

pub async fn list_questions(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(query): Query<ListQuestionsQuery>,
) -> Result<Json<QuestionsResponse>, AppError> {
    let rows = sqlx::query_as::<_, QuestionListRow>(
        r#"
        SELECT
            q.id,
            q.code,
            q.statement,
            q.difficulty,
            q.language,
            (q.external_id IS NOT NULL) AS is_official,
            q.year AS exam_year,
            c.id AS concept_id,
            c.name AS concept_name,
            s.name AS subject_name,
            a.name AS area_name,
            q.source,
            q.license,
            EXISTS (
                SELECT 1 FROM attempts at
                WHERE at.user_id = $1 AND at.question_id = q.id
            ) AS has_attempted,
            (
                SELECT at.is_correct FROM attempts at
                WHERE at.user_id = $1 AND at.question_id = q.id
                ORDER BY at.created_at DESC
                LIMIT 1
            ) AS last_attempt_is_correct
        FROM questions q
        JOIN concepts c ON c.id = q.concept_id
        JOIN subjects s ON s.id = c.subject_id
        JOIN areas a ON a.id = s.area_id
        WHERE ($2::uuid IS NULL OR q.concept_id = $2)
          AND q.external_id IS NOT NULL
        ORDER BY q.code
        "#,
    )
    .bind(auth.id)
    .bind(query.concept_id)
    .fetch_all(&state.pool)
    .await?;

    let questions = rows
        .into_iter()
        .map(|row| QuestionListItem {
            id: row.id,
            code: row.code,
            statement: row.statement,
            difficulty: row.difficulty,
            language: row.language,
            is_official: row.is_official,
            exam_year: row.exam_year,
            concept_id: row.concept_id,
            concept_name: row.concept_name,
            subject_name: row.subject_name,
            area_name: row.area_name,
            source: row.source,
            license: row.license,
            has_attempted: row.has_attempted,
            last_attempt_is_correct: row.last_attempt_is_correct,
        })
        .collect();

    Ok(Json(QuestionsResponse { questions }))
}

pub async fn get_question(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(question_id): Path<Uuid>,
) -> Result<Json<QuestionResponse>, AppError> {
    let row = sqlx::query_as::<_, QuestionDetailRow>(
        r#"
        SELECT
            q.id,
            q.code,
            q.statement,
            q.difficulty,
            q.language,
            (q.external_id IS NOT NULL) AS is_official,
            q.year AS exam_year,
            c.id AS concept_id,
            c.name AS concept_name,
            s.name AS subject_name,
            a.name AS area_name,
            q.source,
            q.license,
            EXISTS (
                SELECT 1 FROM attempts at
                WHERE at.user_id = $1 AND at.question_id = q.id
            ) AS has_attempted,
            (
                SELECT at.is_correct FROM attempts at
                WHERE at.user_id = $1 AND at.question_id = q.id
                ORDER BY at.created_at DESC
                LIMIT 1
            ) AS last_attempt_is_correct,
            q.option_a,
            q.option_b,
            q.option_c,
            q.option_d,
            q.option_e
        FROM questions q
        JOIN concepts c ON c.id = q.concept_id
        JOIN subjects s ON s.id = c.subject_id
        JOIN areas a ON a.id = s.area_id
        WHERE q.id = $2
          AND q.external_id IS NOT NULL
        "#,
    )
    .bind(auth.id)
    .bind(question_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Questão não encontrada.".to_string()))?;

    let assets = sqlx::query_as::<_, QuestionAssetDto>(
        r#"
        SELECT kind, option_label, url, position
        FROM question_assets
        WHERE question_id = $1
        ORDER BY position, id
        "#,
    )
    .bind(question_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(QuestionResponse {
        question: QuestionDetail {
            id: row.id,
            code: row.code,
            statement: row.statement,
            options: vec![
                OptionDto {
                    label: "A".to_string(),
                    text: row.option_a,
                    image_url: option_image(&assets, "A"),
                },
                OptionDto {
                    label: "B".to_string(),
                    text: row.option_b,
                    image_url: option_image(&assets, "B"),
                },
                OptionDto {
                    label: "C".to_string(),
                    text: row.option_c,
                    image_url: option_image(&assets, "C"),
                },
                OptionDto {
                    label: "D".to_string(),
                    text: row.option_d,
                    image_url: option_image(&assets, "D"),
                },
                OptionDto {
                    label: "E".to_string(),
                    text: row.option_e,
                    image_url: option_image(&assets, "E"),
                },
            ],
            difficulty: row.difficulty,
            language: row.language,
            is_official: row.is_official,
            exam_year: row.exam_year,
            assets,
            concept_id: row.concept_id,
            concept_name: row.concept_name,
            subject_name: row.subject_name,
            area_name: row.area_name,
            source: row.source,
            license: row.license,
            has_attempted: row.has_attempted,
            last_attempt_is_correct: row.last_attempt_is_correct,
        },
    }))
}

fn option_image(assets: &[QuestionAssetDto], label: &str) -> Option<String> {
    assets
        .iter()
        .find(|asset| asset.kind == "alternative" && asset.option_label.as_deref() == Some(label))
        .map(|asset| asset.url.clone())
}

pub async fn create_attempt(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(question_id): Path<Uuid>,
    Json(payload): Json<CreateAttempt>,
) -> Result<(StatusCode, Json<AttemptResponse>), AppError> {
    let selected_option = validate_option(&payload.selected_option)?;

    let correct_option = sqlx::query_scalar::<_, String>(
        "SELECT correct_option FROM questions WHERE id = $1 AND external_id IS NOT NULL",
    )
    .bind(question_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Questão não encontrada.".to_string()))?;

    let is_correct = selected_option == correct_option.trim();
    let attempt_id = Uuid::now_v7();

    let created_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        r#"
        INSERT INTO attempts (id, user_id, question_id, selected_option, is_correct)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING created_at
        "#,
    )
    .bind(attempt_id)
    .bind(auth.id)
    .bind(question_id)
    .bind(&selected_option)
    .bind(is_correct)
    .fetch_one(&state.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(AttemptResponse {
            attempt: AttemptDto {
                id: attempt_id,
                is_correct,
                selected_option,
                created_at,
            },
        }),
    ))
}

pub async fn get_discussion(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(question_id): Path<Uuid>,
) -> Result<Json<DiscussionResponse>, AppError> {
    ensure_attempt(&state, auth.id, question_id).await?;

    let comments = sqlx::query_as::<_, CommentDto>(
        r#"
        SELECT
            qc.id,
            u.display_name AS author,
            qc.option_label,
            qc.body,
            qc.created_at,
            (qc.user_id = $1) AS is_own
        FROM question_comments qc
        JOIN users u ON u.id = qc.user_id
        WHERE qc.question_id = $2 AND qc.deleted_at IS NULL
        ORDER BY qc.created_at
        "#,
    )
    .bind(auth.id)
    .bind(question_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(DiscussionResponse {
        question_id,
        comments,
    }))
}

pub async fn create_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(question_id): Path<Uuid>,
    Json(payload): Json<CreateComment>,
) -> Result<(StatusCode, Json<CommentResponse>), AppError> {
    ensure_attempt(&state, auth.id, question_id).await?;

    let body = validate_comment_body(&payload.body)?;
    let option_label = match payload.option_label {
        Some(value) if !value.trim().is_empty() => Some(validate_option(&value)?),
        _ => None,
    };

    let comment_id = Uuid::now_v7();
    let created_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        r#"
        INSERT INTO question_comments (id, question_id, user_id, option_label, body)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING created_at
        "#,
    )
    .bind(comment_id)
    .bind(question_id)
    .bind(auth.id)
    .bind(&option_label)
    .bind(&body)
    .fetch_one(&state.pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(CommentResponse {
            comment: CommentDto {
                id: comment_id,
                author: auth.display_name,
                option_label,
                body,
                created_at,
                is_own: true,
            },
        }),
    ))
}

pub async fn report_comment(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(comment_id): Path<Uuid>,
    Json(payload): Json<ReportComment>,
) -> Result<StatusCode, AppError> {
    let reason = payload.reason.trim();
    if reason.is_empty() || reason.chars().count() > 300 {
        return Err(AppError::BadRequest(
            "Informe um motivo de até 300 caracteres.".to_string(),
        ));
    }

    let comment_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM question_comments WHERE id = $1 AND deleted_at IS NULL)",
    )
    .bind(comment_id)
    .fetch_one(&state.pool)
    .await?;

    if !comment_exists {
        return Err(AppError::NotFound("Comentário não encontrado.".to_string()));
    }

    sqlx::query(
        r#"
        INSERT INTO comment_reports (id, comment_id, user_id, reason)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (comment_id, user_id) DO NOTHING
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(comment_id)
    .bind(auth.id)
    .bind(reason)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_answer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(question_id): Path<Uuid>,
) -> Result<Json<AnswerResponse>, AppError> {
    ensure_attempt(&state, auth.id, question_id).await?;

    let row = sqlx::query_as::<_, (String, String, String, String)>(
        r#"
        SELECT correct_option, explanation, source, license
        FROM questions
        WHERE id = $1 AND external_id IS NOT NULL
        "#,
    )
    .bind(question_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Questão não encontrada.".to_string()))?;

    Ok(Json(AnswerResponse {
        answer: AnswerDto {
            correct_option: row.0.trim().to_string(),
            explanation: row.1,
            source: row.2,
            license: row.3,
        },
    }))
}

pub async fn my_stats(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<StatsResponse>, AppError> {
    let stats = sqlx::query_as::<_, StatsRow>(
        r#"
        SELECT
            count(*)::bigint AS total_attempts,
            count(*) FILTER (WHERE at.is_correct)::bigint AS correct_attempts,
            count(DISTINCT at.question_id)::bigint AS questions_answered
        FROM attempts at
        JOIN questions q ON q.id = at.question_id
        WHERE at.user_id = $1 AND q.external_id IS NOT NULL
        "#,
    )
    .bind(auth.id)
    .fetch_one(&state.pool)
    .await?;

    let accuracy = if stats.total_attempts > 0 {
        stats.correct_attempts as f64 / stats.total_attempts as f64
    } else {
        0.0
    };

    let recent_attempts = sqlx::query_as::<_, RecentAttemptDto>(
        r#"
        SELECT
            q.id AS question_id,
            q.code AS question_code,
            left(q.statement, 140) AS statement_preview,
            at.is_correct,
            at.created_at,
            c.name AS concept_name
        FROM attempts at
        JOIN questions q ON q.id = at.question_id
        JOIN concepts c ON c.id = q.concept_id
        WHERE at.user_id = $1 AND q.external_id IS NOT NULL
        ORDER BY at.created_at DESC
        LIMIT 5
        "#,
    )
    .bind(auth.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(StatsResponse {
        stats: StatsDto {
            total_attempts: stats.total_attempts,
            correct_attempts: stats.correct_attempts,
            questions_answered: stats.questions_answered,
            accuracy,
        },
        recent_attempts,
    }))
}

async fn ensure_attempt(
    state: &AppState,
    user_id: Uuid,
    question_id: Uuid,
) -> Result<(), AppError> {
    let has_attempt = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM attempts at
            JOIN questions q ON q.id = at.question_id
            WHERE at.user_id = $1
              AND at.question_id = $2
              AND q.external_id IS NOT NULL
        )
        "#,
    )
    .bind(user_id)
    .bind(question_id)
    .fetch_one(&state.pool)
    .await?;

    if has_attempt {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
