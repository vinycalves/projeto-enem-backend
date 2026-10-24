use axum::{Json, extract::State};
use serde::Serialize;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize)]
pub struct PixResponse {
    pub key: String,
}

pub async fn pix(State(state): State<AppState>) -> Result<Json<PixResponse>, AppError> {
    let key = state
        .config
        .pix_key
        .clone()
        .ok_or_else(|| AppError::NotFound("Doação não configurada.".to_string()))?;

    Ok(Json(PixResponse { key }))
}
