use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "ENEM Aberto — API de Visualizações",
        version = "0.1.0",
        description = "Catálogo de visualizações interativas, tópicos do ENEM, moléculas e camadas geográficas."
    ),
    paths(
        crate::health::health,
        crate::subjects::list_subjects,
        crate::visualizations::list_visualizations,
        crate::visualizations::get_visualization,
        crate::topics::list_topics,
        crate::molecules::get_molecule,
        crate::geo::get_geo_layer
    ),
    components(schemas(
        crate::health::HealthResponse,
        crate::subjects::SubjectDto,
        crate::subjects::SubjectsResponse,
        crate::visualizations::VisualizationParam,
        crate::visualizations::VisualizationSummary,
        crate::visualizations::VisualizationTopicDto,
        crate::visualizations::VisualizationDetail,
        crate::visualizations::VisualizationListResponse,
        crate::visualizations::VisualizationResponse,
        crate::topics::TopicDto,
        crate::topics::SubjectTopicsDto,
        crate::topics::TopicsResponse,
        crate::molecules::MoleculeDto,
        crate::molecules::MoleculeResponse,
        crate::geo::GeoLayerDto,
        crate::geo::GeoLayerResponse
    ))
)]
pub struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
