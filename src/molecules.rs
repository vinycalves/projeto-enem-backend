use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct MoleculeDto {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub formula: String,
    #[schema(value_type = Object)]
    pub atoms: serde_json::Value,
    #[schema(value_type = Object)]
    pub bonds: serde_json::Value,
    #[schema(value_type = Object)]
    pub metadata: serde_json::Value,
    #[schema(value_type = Vec<String>)]
    pub sources: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MoleculeResponse {
    pub molecule: MoleculeDto,
}

#[utoipa::path(
    get,
    path = "/api/v1/molecules/{slug}",
    params(("slug" = String, Path, description = "Slug da molécula")),
    responses(
        (status = 200, body = MoleculeResponse),
        (status = 404, description = "Molécula não encontrada")
    )
)]
pub async fn get_molecule(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<MoleculeResponse>, AppError> {
    let molecule = sqlx::query_as::<_, MoleculeDto>(
        r#"
        SELECT id, slug, name, formula, atoms, bonds, metadata, sources
        FROM molecules
        WHERE slug = $1
        "#,
    )
    .bind(slug)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Molécula não encontrada.".to_string()))?;

    Ok(Json(MoleculeResponse { molecule }))
}
