use std::collections::{HashMap, HashSet};

use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::{error::AppError, state::AppState};

const REACTIONS_SEED: &str = include_str!("../seeds/chemistry/reactions.json");
const DATASETS_SEED: &str = include_str!("../seeds/chemistry/datasets.json");
const ELEMENTS_SEED: &str = include_str!("../seeds/chemistry/elements.json");

#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ElementSeed {
    pub atomic_number: i32,
    pub symbol: String,
    pub name_pt: String,
    pub atomic_mass: f64,
    pub group_number: Option<i32>,
    pub period: i32,
    pub electronegativity: Option<f64>,
    pub atomic_radius_pm: Option<f64>,
    pub electron_configuration: String,
    pub category: String,
    pub standard_state: Option<String>,
    pub cpk_hex: Option<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ElementsResponse {
    pub elements: Vec<ElementSeed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AtomSeed {
    pub id: String,
    pub element: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReactionSpeciesSeed {
    pub side: String,
    pub formula: String,
    pub name: String,
    pub state: String,
    pub coefficient: u32,
    pub charge: i32,
    pub atoms: Vec<AtomSeed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AtomMappingSeed {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReactionStageSeed {
    pub id: String,
    pub from: f64,
    pub to: f64,
    pub legend: String,
    pub breaking_bonds: Vec<String>,
    pub forming_bonds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ThermodynamicsSeed {
    pub delta_h_kj_mol: f64,
    pub conditions: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReactionSeed {
    pub id: String,
    pub name: String,
    pub classification: String,
    pub equation: String,
    pub species: Vec<ReactionSpeciesSeed>,
    pub atom_mapping: Vec<AtomMappingSeed>,
    pub stages: Vec<ReactionStageSeed>,
    pub thermodynamics: Option<ThermodynamicsSeed>,
    pub enem_topics: Vec<String>,
    pub didactic_note: String,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DatasetSeed {
    pub name: String,
    pub description: String,
    pub conditions: String,
    pub sources: Vec<String>,
    #[schema(value_type = Object)]
    pub data: Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChemistryReactionResponse {
    pub reaction: ReactionSeed,
}

#[derive(Debug, sqlx::FromRow)]
struct ElementRow {
    atomic_number: i32,
    symbol: String,
    name_pt: String,
    atomic_mass: f64,
    group_number: Option<i32>,
    period: i32,
    electronegativity: Option<f64>,
    atomic_radius_pm: Option<f64>,
    electron_configuration: String,
    category: String,
    standard_state: Option<String>,
    cpk_hex: Option<String>,
    sources: Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChemistryDatasetResponse {
    pub dataset: ChemistryDatasetDto,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChemistryDatasetDto {
    pub name: String,
    pub description: String,
    pub conditions: String,
    #[schema(value_type = Object)]
    pub data: Value,
    pub sources: Vec<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct ReactionRow {
    reaction: Value,
}

#[derive(Debug, sqlx::FromRow)]
struct DatasetRow {
    name: String,
    description: String,
    conditions: String,
    data: Value,
    sources: Value,
}

pub fn reaction_seeds() -> Result<Vec<ReactionSeed>, String> {
    serde_json::from_str(REACTIONS_SEED)
        .map_err(|error| format!("seed de reações inválido: {error}"))
}

pub fn dataset_seeds() -> Result<Vec<DatasetSeed>, String> {
    serde_json::from_str(DATASETS_SEED)
        .map_err(|error| format!("seed de datasets inválido: {error}"))
}

pub fn element_seeds() -> Result<Vec<ElementSeed>, String> {
    serde_json::from_str(ELEMENTS_SEED)
        .map_err(|error| format!("seed de elementos inválido: {error}"))
}

pub fn validate_elements(elements: &[ElementSeed]) -> Result<(), String> {
    if elements.is_empty() {
        return Err("seed de elementos vazio".to_string());
    }
    let mut numbers = HashSet::new();
    let mut symbols = HashSet::new();
    for element in elements {
        if !(1..=118).contains(&element.atomic_number) {
            return Err(format!(
                "número atômico inválido: {}",
                element.atomic_number
            ));
        }
        if !numbers.insert(element.atomic_number) {
            return Err(format!(
                "número atômico duplicado: {}",
                element.atomic_number
            ));
        }
        if element.symbol.trim().is_empty() || element.name_pt.trim().is_empty() {
            return Err(format!(
                "elemento {} sem símbolo ou nome",
                element.atomic_number
            ));
        }
        if !symbols.insert(element.symbol.as_str()) {
            return Err(format!("símbolo duplicado: {}", element.symbol));
        }
        if !element.atomic_mass.is_finite() || element.atomic_mass <= 0.0 {
            return Err(format!("massa atômica inválida: {}", element.symbol));
        }
        if !(1..=7).contains(&element.period) {
            return Err(format!("período inválido: {}", element.symbol));
        }
        if let Some(group) = element.group_number
            && !(1..=18).contains(&group)
        {
            return Err(format!("grupo inválido: {}", element.symbol));
        }
        if element.sources.is_empty() {
            return Err(format!("elemento {} sem fontes", element.symbol));
        }
    }
    Ok(())
}

pub fn validate_reaction(reaction: &ReactionSeed) -> Result<(), String> {
    if reaction.id.trim().is_empty() || reaction.name.trim().is_empty() {
        return Err("reação sem id ou nome".to_string());
    }
    if reaction.species.is_empty() {
        return Err(format!("{}: reação sem espécies", reaction.id));
    }
    if reaction.equation.trim().is_empty() {
        return Err(format!("{}: reação sem equação simbólica", reaction.id));
    }
    if reaction.stages.is_empty() {
        return Err(format!("{}: reação sem estágios didáticos", reaction.id));
    }
    if reaction.sources.is_empty() {
        return Err(format!("{}: reação sem fontes", reaction.id));
    }
    if reaction.enem_topics.is_empty() || reaction.didactic_note.trim().is_empty() {
        return Err(format!(
            "{}: reação sem conexão ENEM ou nota didática",
            reaction.id
        ));
    }

    let mut atom_sides: HashMap<&str, (&str, &str)> = HashMap::new();
    let mut reactant_elements: HashMap<&str, u32> = HashMap::new();
    let mut product_elements: HashMap<&str, u32> = HashMap::new();
    let mut reactant_charge = 0i64;
    let mut product_charge = 0i64;
    let mut coefficients = Vec::new();

    for species in &reaction.species {
        if species.coefficient == 0 {
            return Err(format!(
                "{}: coeficiente zero em {}",
                reaction.id, species.formula
            ));
        }
        if species.side != "reactant" && species.side != "product" {
            return Err(format!(
                "{}: lado inválido em {}",
                reaction.id, species.formula
            ));
        }
        if species.atoms.is_empty() {
            return Err(format!(
                "{}: espécie sem átomos: {}",
                reaction.id, species.formula
            ));
        }
        let formula_counts = parse_formula_counts(&species.formula)
            .ok_or_else(|| format!("{}: fórmula inválida: {}", reaction.id, species.formula))?;
        let actual_counts = count_atom_elements(&species.atoms);
        for (element, count) in &formula_counts {
            let expected = count * species.coefficient;
            if actual_counts.get(element).copied().unwrap_or(0) != expected {
                return Err(format!(
                    "{}: {} tem {} átomos de {}, esperado {} (coeficiente {})",
                    reaction.id,
                    species.formula,
                    actual_counts.get(element).copied().unwrap_or(0),
                    element,
                    expected,
                    species.coefficient
                ));
            }
        }
        if actual_counts
            .keys()
            .any(|element| !formula_counts.contains_key(element))
        {
            return Err(format!(
                "{}: {} contém elemento fora da fórmula",
                reaction.id, species.formula
            ));
        }
        coefficients.push(species.coefficient);

        for atom in &species.atoms {
            if atom.element.trim().is_empty() {
                return Err(format!("{}: átomo sem elemento", reaction.id));
            }
            if atom_sides
                .insert(
                    atom.id.as_str(),
                    (species.side.as_str(), atom.element.as_str()),
                )
                .is_some()
            {
                return Err(format!(
                    "{}: id de átomo duplicado: {}",
                    reaction.id, atom.id
                ));
            }

            let counter = if species.side == "reactant" {
                &mut reactant_elements
            } else {
                &mut product_elements
            };
            *counter.entry(atom.element.as_str()).or_default() += 1;
        }

        let charge = i64::from(species.charge) * i64::from(species.coefficient);
        if species.side == "reactant" {
            reactant_charge += charge;
        } else {
            product_charge += charge;
        }
    }

    if reactant_elements != product_elements {
        return Err(format!(
            "{}: conservação de átomos violada: reagentes {:?}, produtos {:?}",
            reaction.id, reactant_elements, product_elements
        ));
    }
    if reactant_charge != product_charge {
        return Err(format!(
            "{}: conservação de carga violada: {} != {}",
            reaction.id, reactant_charge, product_charge
        ));
    }
    if gcd_of(&coefficients) != 1 {
        return Err(format!(
            "{}: coeficientes não estão na proporção inteira mínima",
            reaction.id
        ));
    }

    let reactant_atom_count = atom_sides
        .values()
        .filter(|(side, _)| *side == "reactant")
        .count();
    let product_atom_count = atom_sides
        .values()
        .filter(|(side, _)| *side == "product")
        .count();
    if reaction.atom_mapping.len() != reactant_atom_count
        || reaction.atom_mapping.len() != product_atom_count
    {
        return Err(format!(
            "{}: atomMapping deve cobrir todos os átomos ({} reagentes, {} produtos, {} mapeados)",
            reaction.id,
            reactant_atom_count,
            product_atom_count,
            reaction.atom_mapping.len()
        ));
    }

    let mut mapped_from = HashSet::new();
    let mut mapped_to = HashSet::new();
    for mapping in &reaction.atom_mapping {
        if !mapped_from.insert(mapping.from.as_str()) {
            return Err(format!(
                "{}: átomo de reagente mapeado duas vezes: {}",
                reaction.id, mapping.from
            ));
        }
        if !mapped_to.insert(mapping.to.as_str()) {
            return Err(format!(
                "{}: átomo de produto mapeado duas vezes: {}",
                reaction.id, mapping.to
            ));
        }
        let from = atom_sides.get(mapping.from.as_str()).ok_or_else(|| {
            format!(
                "{}: átomo de origem inexistente: {}",
                reaction.id, mapping.from
            )
        })?;
        let to = atom_sides.get(mapping.to.as_str()).ok_or_else(|| {
            format!(
                "{}: átomo de destino inexistente: {}",
                reaction.id, mapping.to
            )
        })?;
        if from.0 != "reactant" || to.0 != "product" {
            return Err(format!(
                "{}: mapeamento deve ir de reagente para produto",
                reaction.id
            ));
        }
        if from.1 != to.1 {
            return Err(format!(
                "{}: mapeamento entre elementos diferentes: {} -> {}",
                reaction.id, mapping.from, mapping.to
            ));
        }
    }

    for stage in &reaction.stages {
        if !(0.0..=1.0).contains(&stage.from)
            || !(0.0..=1.0).contains(&stage.to)
            || stage.from >= stage.to
        {
            return Err(format!(
                "{}: estágio com intervalo inválido: {}",
                reaction.id, stage.id
            ));
        }
    }

    if let Some(thermo) = &reaction.thermodynamics {
        if !thermo.delta_h_kj_mol.is_finite() {
            return Err(format!("{}: entalpia não finita", reaction.id));
        }
        if thermo.source.trim().is_empty() || thermo.conditions.trim().is_empty() {
            return Err(format!(
                "{}: termodinâmica sem fonte ou condições",
                reaction.id
            ));
        }
    }

    Ok(())
}

pub fn validate_dataset(dataset: &DatasetSeed) -> Result<(), String> {
    if dataset.name.trim().is_empty() {
        return Err("dataset sem nome".to_string());
    }
    if dataset.sources.is_empty() {
        return Err(format!("{}: dataset sem fontes", dataset.name));
    }
    if !dataset.data.is_object() {
        return Err(format!("{}: data deve ser um objeto JSON", dataset.name));
    }
    Ok(())
}

pub async fn sync_seeds(pool: &PgPool) -> Result<(), SeedError> {
    let elements = element_seeds().map_err(SeedError::Invalid)?;
    validate_elements(&elements).map_err(SeedError::Invalid)?;
    for element in &elements {
        let sources = serde_json::to_value(&element.sources)?;
        sqlx::query(
            r#"
            INSERT INTO chemistry_elements (
                atomic_number, symbol, name_pt, atomic_mass, group_number, period,
                electronegativity, atomic_radius_pm, electron_configuration, category,
                standard_state, cpk_hex, sources
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (atomic_number) DO UPDATE SET
                symbol = EXCLUDED.symbol,
                name_pt = EXCLUDED.name_pt,
                atomic_mass = EXCLUDED.atomic_mass,
                group_number = EXCLUDED.group_number,
                period = EXCLUDED.period,
                electronegativity = EXCLUDED.electronegativity,
                atomic_radius_pm = EXCLUDED.atomic_radius_pm,
                electron_configuration = EXCLUDED.electron_configuration,
                category = EXCLUDED.category,
                standard_state = EXCLUDED.standard_state,
                cpk_hex = EXCLUDED.cpk_hex,
                sources = EXCLUDED.sources,
                updated_at = now()
            "#,
        )
        .bind(element.atomic_number)
        .bind(&element.symbol)
        .bind(&element.name_pt)
        .bind(element.atomic_mass)
        .bind(element.group_number)
        .bind(element.period)
        .bind(element.electronegativity)
        .bind(element.atomic_radius_pm)
        .bind(&element.electron_configuration)
        .bind(&element.category)
        .bind(&element.standard_state)
        .bind(&element.cpk_hex)
        .bind(sources)
        .execute(pool)
        .await?;
    }

    let reactions = reaction_seeds().map_err(SeedError::Invalid)?;
    let mut reaction_ids = HashSet::new();
    for reaction in &reactions {
        validate_reaction(reaction).map_err(SeedError::Invalid)?;
        if !reaction_ids.insert(reaction.id.as_str()) {
            return Err(SeedError::Invalid(format!(
                "id de reação duplicado: {}",
                reaction.id
            )));
        }
        let payload = serde_json::to_value(reaction)?;
        let sources = serde_json::to_value(&reaction.sources)?;
        sqlx::query(
            r#"
            INSERT INTO chemistry_reactions (id, name, classification, reaction, sources)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                classification = EXCLUDED.classification,
                reaction = EXCLUDED.reaction,
                sources = EXCLUDED.sources
            "#,
        )
        .bind(&reaction.id)
        .bind(&reaction.name)
        .bind(&reaction.classification)
        .bind(payload)
        .bind(sources)
        .execute(pool)
        .await?;
    }

    let datasets = dataset_seeds().map_err(SeedError::Invalid)?;
    let mut dataset_names = HashSet::new();
    for dataset in &datasets {
        validate_dataset(dataset).map_err(SeedError::Invalid)?;
        if !dataset_names.insert(dataset.name.as_str()) {
            return Err(SeedError::Invalid(format!(
                "nome de dataset duplicado: {}",
                dataset.name
            )));
        }
        let sources = serde_json::to_value(&dataset.sources)?;
        sqlx::query(
            r#"
            INSERT INTO chemistry_datasets (name, description, conditions, data, sources)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (name) DO UPDATE SET
                description = EXCLUDED.description,
                conditions = EXCLUDED.conditions,
                data = EXCLUDED.data,
                sources = EXCLUDED.sources,
                updated_at = now()
            "#,
        )
        .bind(&dataset.name)
        .bind(&dataset.description)
        .bind(&dataset.conditions)
        .bind(&dataset.data)
        .bind(sources)
        .execute(pool)
        .await?;
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/chemistry/elements",
    responses((status = 200, body = ElementsResponse))
)]
pub async fn get_elements(
    State(state): State<AppState>,
) -> Result<Json<ElementsResponse>, AppError> {
    let rows = sqlx::query_as::<_, ElementRow>(
        r#"
        SELECT atomic_number, symbol, name_pt, atomic_mass, group_number, period,
               electronegativity, atomic_radius_pm, electron_configuration, category,
               standard_state, cpk_hex, sources
        FROM chemistry_elements
        ORDER BY atomic_number
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    let elements = rows
        .into_iter()
        .map(|row| {
            let sources: Vec<String> = serde_json::from_value(row.sources).map_err(|error| {
                tracing::error!(%error, "fontes inválidas em elemento químico");
                AppError::Internal
            })?;
            Ok(ElementSeed {
                atomic_number: row.atomic_number,
                symbol: row.symbol,
                name_pt: row.name_pt,
                atomic_mass: row.atomic_mass,
                group_number: row.group_number,
                period: row.period,
                electronegativity: row.electronegativity,
                atomic_radius_pm: row.atomic_radius_pm,
                electron_configuration: row.electron_configuration,
                category: row.category,
                standard_state: row.standard_state,
                cpk_hex: row.cpk_hex,
                sources,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(ElementsResponse { elements }))
}

#[utoipa::path(
    get,
    path = "/api/v1/chemistry/reactions/{id}",
    params(("id" = String, Path, description = "Identificador da reação")),
    responses(
        (status = 200, body = ChemistryReactionResponse),
        (status = 404, description = "Reação não encontrada")
    )
)]
pub async fn get_reaction(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ChemistryReactionResponse>, AppError> {
    let row =
        sqlx::query_as::<_, ReactionRow>("SELECT reaction FROM chemistry_reactions WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Reação química não encontrada.".to_string()))?;

    let reaction: ReactionSeed = serde_json::from_value(row.reaction).map_err(|error| {
        tracing::error!(%error, "reação inválida no banco");
        AppError::Internal
    })?;
    Ok(Json(ChemistryReactionResponse { reaction }))
}

#[utoipa::path(
    get,
    path = "/api/v1/chemistry/datasets/{name}",
    params(("name" = String, Path, description = "Nome do dataset químico versionado")),
    responses(
        (status = 200, body = ChemistryDatasetResponse),
        (status = 404, description = "Dataset não encontrado")
    )
)]
pub async fn get_dataset(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ChemistryDatasetResponse>, AppError> {
    let row = sqlx::query_as::<_, DatasetRow>(
        r#"
        SELECT name, description, conditions, data, sources
        FROM chemistry_datasets
        WHERE name = $1
        "#,
    )
    .bind(name)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Dataset químico não encontrado.".to_string()))?;

    let sources: Vec<String> = serde_json::from_value(row.sources).map_err(|error| {
        tracing::error!(%error, "fontes inválidas no dataset");
        AppError::Internal
    })?;

    Ok(Json(ChemistryDatasetResponse {
        dataset: ChemistryDatasetDto {
            name: row.name,
            description: row.description,
            conditions: row.conditions,
            data: row.data,
            sources,
        },
    }))
}

fn count_atom_elements(atoms: &[AtomSeed]) -> HashMap<String, u32> {
    let mut counts = HashMap::new();
    for atom in atoms {
        *counts.entry(atom.element.clone()).or_default() += 1;
    }
    counts
}

fn parse_formula_counts(formula: &str) -> Option<HashMap<String, u32>> {
    let characters: Vec<char> = formula.chars().collect();
    let mut stack: Vec<HashMap<String, u32>> = vec![HashMap::new()];
    let mut index = 0;

    while index < characters.len() {
        let character = characters[index];
        if character == '(' {
            stack.push(HashMap::new());
            index += 1;
            continue;
        }
        if character == ')' {
            index += 1;
            let multiplier = parse_multiplier(&characters, &mut index);
            let group = stack.pop()?;
            let target = stack.last_mut()?;
            for (element, count) in group {
                *target.entry(element).or_default() += count * multiplier;
            }
            continue;
        }
        if character.is_ascii_uppercase() {
            let mut element = String::from(character);
            index += 1;
            while index < characters.len() && characters[index].is_ascii_lowercase() {
                element.push(characters[index]);
                index += 1;
            }
            let multiplier = parse_multiplier(&characters, &mut index);
            *stack.last_mut()?.entry(element).or_default() += multiplier;
            continue;
        }
        if character.is_whitespace() || character == '·' || character == '.' {
            index += 1;
            continue;
        }
        return None;
    }

    if stack.len() != 1 {
        return None;
    }
    stack.pop()
}

fn parse_multiplier(characters: &[char], index: &mut usize) -> u32 {
    let start = *index;
    let mut value = 0u32;
    while *index < characters.len() && characters[*index].is_ascii_digit() {
        value = value * 10 + (characters[*index] as u32 - u32::from(b'0'));
        *index += 1;
    }
    if *index == start { 1 } else { value }
}

fn gcd_of(values: &[u32]) -> u32 {
    values.iter().copied().fold(0, gcd)
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_elements_are_valid() {
        let elements = element_seeds().expect("seed deve carregar");
        validate_elements(&elements).expect("elementos devem ser válidos");
        assert_eq!(elements.len(), 118);
    }

    #[test]
    fn rejects_duplicate_element_symbol() {
        let mut elements = element_seeds().expect("seed deve carregar");
        elements[1].symbol = elements[0].symbol.clone();
        assert!(validate_elements(&elements).is_err());
    }

    #[test]
    fn seeded_reactions_are_valid() {
        for reaction in reaction_seeds().expect("seed deve carregar") {
            validate_reaction(&reaction).unwrap_or_else(|error| panic!("{error}"));
        }
    }

    #[test]
    fn seeded_datasets_are_valid() {
        for dataset in dataset_seeds().expect("seed deve carregar") {
            validate_dataset(&dataset).unwrap_or_else(|error| panic!("{error}"));
        }
    }

    #[test]
    fn rejects_unbalanced_reaction() {
        let mut reaction = reaction_seeds().expect("seed deve carregar")[0].clone();
        reaction.species[2].atoms.pop();
        assert!(validate_reaction(&reaction).is_err());
    }

    #[test]
    fn rejects_non_minimal_coefficients() {
        let mut reaction = reaction_seeds().expect("seed deve carregar")[0].clone();
        for species in &mut reaction.species {
            species.coefficient *= 2;
        }
        assert!(validate_reaction(&reaction).is_err());
    }

    #[test]
    fn rejects_invalid_atom_mapping() {
        let mut reaction = reaction_seeds().expect("seed deve carregar")[0].clone();
        reaction.atom_mapping[0].to = "CO2:O1".to_string();
        assert!(validate_reaction(&reaction).is_err());
    }

    #[test]
    fn rejects_wrong_charge() {
        let mut reaction = reaction_seeds().expect("seed deve carregar")[0].clone();
        reaction.species[2].charge = 1;
        assert!(validate_reaction(&reaction).is_err());
    }

    #[test]
    fn parses_formula_composition() {
        let counts = parse_formula_counts("Ca(OH)2").expect("fórmula válida");
        assert_eq!(counts.get("Ca"), Some(&1));
        assert_eq!(counts.get("O"), Some(&2));
        assert_eq!(counts.get("H"), Some(&2));

        let counts = parse_formula_counts("N2O4").expect("fórmula válida");
        assert_eq!(counts.get("N"), Some(&2));
        assert_eq!(counts.get("O"), Some(&4));
    }

    #[test]
    fn rejects_atoms_inconsistent_with_formula() {
        let mut reaction = reaction_seeds().expect("seed deve carregar")[0].clone();
        reaction.species[0].atoms.pop();
        assert!(validate_reaction(&reaction).is_err());
    }
}
