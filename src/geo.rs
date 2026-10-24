use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct GeoLayerDto {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    #[schema(value_type = Object)]
    pub geojson: serde_json::Value,
    #[schema(value_type = Object)]
    pub metadata: serde_json::Value,
    #[schema(value_type = Vec<String>)]
    pub sources: serde_json::Value,
    pub level: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GeoLayerResponse {
    pub layer: GeoLayerDto,
}

#[utoipa::path(
    get,
    path = "/api/v1/geo/layers/{layer}",
    params(("layer" = String, Path, description = "Slug da camada geográfica")),
    responses(
        (status = 200, body = GeoLayerResponse),
        (status = 404, description = "Camada não encontrada")
    )
)]
pub async fn get_geo_layer(
    State(state): State<AppState>,
    Path(layer): Path<String>,
) -> Result<Json<GeoLayerResponse>, AppError> {
    let layer = sqlx::query_as::<_, GeoLayerDto>(
        r#"
        SELECT id, slug, name, geojson, metadata, sources, level
        FROM geo_layers
        WHERE slug = $1
        "#,
    )
    .bind(layer)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Camada geográfica não encontrada.".to_string()))?;

    Ok(Json(GeoLayerResponse { layer }))
}
