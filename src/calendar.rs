use axum::{Json, extract::State};
use chrono::NaiveDate;
use serde::Serialize;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EventDto {
    pub id: Uuid,
    pub title: String,
    pub kind: String,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    pub description: String,
    pub source_url: String,
    pub confirmed: bool,
}

#[derive(Debug, Serialize)]
pub struct CalendarResponse {
    pub events: Vec<EventDto>,
}

pub async fn list_events(
    State(state): State<AppState>,
) -> Result<Json<CalendarResponse>, AppError> {
    let events = sqlx::query_as::<_, EventDto>(
        r#"
        SELECT
            id,
            title,
            kind,
            starts_on,
            ends_on,
            description,
            source_url,
            confirmed
        FROM enem_events
        ORDER BY starts_on NULLS LAST, sort_order, title
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(CalendarResponse { events }))
}
