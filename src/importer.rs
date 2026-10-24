use std::{collections::HashMap, error::Error, time::Duration};

use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

const API_BASE: &str = "https://api.enem.dev/v1";
const PAGE_SIZE: usize = 50;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiResponse {
    metadata: ApiMetadata,
    questions: Vec<ApiQuestion>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMetadata {
    total: i64,
    has_more: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ApiQuestion {
    title: String,
    index: i64,
    discipline: Option<String>,
    language: Option<String>,
    year: i64,
    context: Option<String>,
    files: Vec<String>,
    correct_alternative: String,
    alternatives_introduction: Option<String>,
    alternatives: Vec<ApiAlternative>,
}

#[derive(Debug, Deserialize, Clone)]
struct ApiAlternative {
    letter: String,
    text: Option<String>,
    file: Option<String>,
}

#[derive(Debug, Default)]
pub struct ImportStats {
    pub fetched: usize,
    pub upserted: usize,
    pub skipped: usize,
    pub errors: usize,
}

pub async fn run_import(
    pool: Option<&PgPool>,
    client: &Client,
    from_year: i32,
    to_year: i32,
    limit: Option<usize>,
) -> Result<ImportStats, Box<dyn Error>> {
    let mut stats = ImportStats::default();
    let mut concept_cache: HashMap<(String, i32), Uuid> = HashMap::new();

    for year in from_year..=to_year {
        // The no-language request returns all questions on older exams and the
        // default language version on newer ones. It is the safest base.
        let base = fetch_year(client, year, None, limit).await?;
        stats.fetched += base.len();
        import_questions(pool, &mut stats, &mut concept_cache, year, &base).await;

        // The API repeats common questions in the English response; only the
        // language-specific English questions are new here.
        let english = fetch_year(client, year, Some("ingles"), limit).await?;
        stats.fetched += english.len();
        let only_english = english
            .into_iter()
            .filter(|question| question.language.as_deref() == Some("ingles"))
            .collect::<Vec<_>>();
        import_questions(pool, &mut stats, &mut concept_cache, year, &only_english).await;

        tracing::info!(
            year,
            fetched = stats.fetched,
            upserted = stats.upserted,
            "ano processado"
        );
    }

    Ok(stats)
}

async fn import_questions(
    pool: Option<&PgPool>,
    stats: &mut ImportStats,
    concept_cache: &mut HashMap<(String, i32), Uuid>,
    year: i32,
    questions: &[ApiQuestion],
) {
    for question in questions {
        if pool.is_none() {
            stats.upserted += 1;
            continue;
        }

        let discipline = question.discipline.as_deref().unwrap_or_default();
        let (area_code, area_name) = area_info(discipline);
        let cache_key = (area_code.to_string(), year);

        let concept_id = match concept_cache.get(&cache_key) {
            Some(id) => *id,
            None => {
                match ensure_official_concept(pool.unwrap(), area_code, area_name, year).await {
                    Ok(id) => {
                        concept_cache.insert(cache_key, id);
                        id
                    }
                    Err(error) => {
                        stats.errors += 1;
                        tracing::warn!(%error, year, area = area_code, "falha ao criar conceito oficial");
                        continue;
                    }
                }
            }
        };

        match import_question(pool.unwrap(), concept_id, question).await {
            Ok(()) => stats.upserted += 1,
            Err(error) => {
                stats.errors += 1;
                tracing::warn!(%error, year, index = question.index, "falha ao importar questão");
            }
        }
    }
}

async fn fetch_year(
    client: &Client,
    year: i32,
    language: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<ApiQuestion>, Box<dyn Error>> {
    let mut offset = 0usize;
    let mut questions = Vec::new();

    loop {
        let mut url =
            format!("{API_BASE}/exams/{year}/questions?limit={PAGE_SIZE}&offset={offset}");
        if let Some(language) = language {
            url.push_str(&format!("&language={language}"));
        }

        let mut attempts = 0;
        let response = loop {
            let response = client.get(&url).send().await?;
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS && attempts < 5 {
                attempts += 1;
                tracing::warn!(
                    year,
                    ?language,
                    attempt = attempts,
                    "rate limit; aguardando"
                );
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            break response;
        };

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            tracing::info!(year, "prova não encontrada na API");
            break;
        }

        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(
                year,
                ?language,
                %status,
                body = %body.chars().take(180).collect::<String>(),
                "requisição ignorada"
            );
            break;
        }

        if !response.status().is_success() {
            return Err(format!("API retornou {} para {}", response.status(), url).into());
        }

        let payload: ApiResponse = response.json().await?;
        let page_len = payload.questions.len();
        tracing::debug!(
            year,
            ?language,
            total = payload.metadata.total,
            page = page_len,
            "página recebida"
        );

        questions.extend(payload.questions);

        if let Some(limit) = limit
            && questions.len() >= limit
        {
            questions.truncate(limit);
            break;
        }

        if page_len == 0 || !payload.metadata.has_more {
            break;
        }

        offset += page_len;
        tokio::time::sleep(Duration::from_millis(650)).await;
    }

    Ok(questions)
}

async fn ensure_official_concept(
    pool: &PgPool,
    area_code: &str,
    area_name: &str,
    year: i32,
) -> Result<Uuid, Box<dyn Error>> {
    let subject_code = format!("oficial-{area_code}");
    let subject_name = format!("Questões oficiais — {area_name}");
    let subject_description = "Questões de provas anteriores do ENEM, organizadas por ano e área.";

    let subject_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO subjects (id, area_id, code, name, description, sort_order)
        VALUES (
            $1,
            (SELECT id FROM areas WHERE code = $2),
            $3,
            $4,
            $5,
            100
        )
        ON CONFLICT (code) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(area_code)
    .bind(&subject_code)
    .bind(&subject_name)
    .bind(subject_description)
    .fetch_one(pool)
    .await?;

    let concept_code = format!("oficial-{area_code}-{year}");
    let concept_name = format!("ENEM {year}");
    let summary = format!("Questões oficiais do ENEM {year}.");
    let explanation = format!(
        "Questões extraídas dos cadernos oficiais do INEP e disponibilizadas via enem.dev. \
         Responda, compare raciocínios na discussão e consulte o gabarito e a fonte depois da tentativa. \
         Prova aplicada em {year}."
    );

    let concept_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO concepts (id, subject_id, code, name, summary, explanation, difficulty, sort_order)
        VALUES ($1, $2, $3, $4, $5, $6, 'medio', $7)
        ON CONFLICT (code) DO UPDATE SET name = EXCLUDED.name
        RETURNING id
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(subject_id)
    .bind(&concept_code)
    .bind(&concept_name)
    .bind(&summary)
    .bind(&explanation)
    .bind(year)
    .fetch_one(pool)
    .await?;

    Ok(concept_id)
}

async fn import_question(
    pool: &PgPool,
    concept_id: Uuid,
    question: &ApiQuestion,
) -> Result<(), Box<dyn Error>> {
    let correct_option = question.correct_alternative.trim().to_uppercase();
    if !matches!(correct_option.as_str(), "A" | "B" | "C" | "D" | "E") {
        return Err("alternativa correta inválida".into());
    }

    let language = question.language.as_deref().unwrap_or("comum");
    let external_id = external_id(question);
    let code = question_code(question);
    let statement = build_statement(question);
    let (option_a, option_b, option_c, option_d, option_e) = build_options(&question.alternatives);

    let source = format!("ENEM {} — INEP (via enem.dev)", question.year);
    let license = "Dados públicos do INEP — via enem.dev; consulte a fonte oficial".to_string();
    let explanation = format!(
        "Gabarito oficial: alternativa {correct_option}. Fonte: INEP/ENEM {}, disponibilizada via enem.dev.",
        question.year
    );

    let mut tx = pool.begin().await?;

    let question_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO questions (
            id, concept_id, code, statement,
            option_a, option_b, option_c, option_d, option_e,
            correct_option, explanation, difficulty, source, license, year,
            language, external_id
        )
        VALUES (
            $1, $2, $3, $4,
            $5, $6, $7, $8, $9,
            $10, $11, 'medio', $12, $13, $14,
            $15, $16
        )
        ON CONFLICT (external_id) DO UPDATE SET
            concept_id = EXCLUDED.concept_id,
            statement = EXCLUDED.statement,
            option_a = EXCLUDED.option_a,
            option_b = EXCLUDED.option_b,
            option_c = EXCLUDED.option_c,
            option_d = EXCLUDED.option_d,
            option_e = EXCLUDED.option_e,
            correct_option = EXCLUDED.correct_option,
            explanation = EXCLUDED.explanation,
            source = EXCLUDED.source,
            license = EXCLUDED.license,
            year = EXCLUDED.year,
            language = EXCLUDED.language
        RETURNING id
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(concept_id)
    .bind(&code)
    .bind(&statement)
    .bind(&option_a)
    .bind(&option_b)
    .bind(&option_c)
    .bind(&option_d)
    .bind(&option_e)
    .bind(&correct_option)
    .bind(&explanation)
    .bind(&source)
    .bind(&license)
    .bind(question.year)
    .bind(language)
    .bind(&external_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM question_assets WHERE question_id = $1")
        .bind(question_id)
        .execute(&mut *tx)
        .await?;

    for (position, url) in question.files.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO question_assets (id, question_id, kind, option_label, url, position)
            VALUES ($1, $2, 'context', NULL, $3, $4)
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(question_id)
        .bind(url)
        .bind(position as i32)
        .execute(&mut *tx)
        .await?;
    }

    for alternative in &question.alternatives {
        if let Some(file) = &alternative.file {
            sqlx::query(
                r#"
                INSERT INTO question_assets (id, question_id, kind, option_label, url, position)
                VALUES ($1, $2, 'alternative', $3, $4, $5)
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(question_id)
            .bind(alternative.letter.trim().to_uppercase())
            .bind(file)
            .bind(alternative_letter_position(&alternative.letter))
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

pub fn area_info(discipline: &str) -> (&'static str, &'static str) {
    let normalized = discipline.to_lowercase();
    if normalized.contains("linguagens") {
        ("linguagens", "Linguagens")
    } else if normalized.contains("humanas") {
        ("humanas", "Ciências Humanas")
    } else if normalized.contains("natureza") {
        ("natureza", "Ciências da Natureza")
    } else if normalized.contains("matematica") || normalized.contains("matemática") {
        ("matematica", "Matemática")
    } else {
        ("linguagens", "Linguagens")
    }
}

fn external_id(question: &ApiQuestion) -> String {
    format!(
        "enem.dev:{}:{}:{}",
        question.year,
        question.index,
        question
            .language
            .as_deref()
            .unwrap_or("comum")
            .to_lowercase()
    )
}

fn question_code(question: &ApiQuestion) -> String {
    match question.language.as_deref() {
        Some(language) if language != "comum" => {
            format!(
                "ENEM-{}-{:03}-{}",
                question.year,
                question.index,
                language.to_uppercase()
            )
        }
        _ => format!("ENEM-{}-{:03}", question.year, question.index),
    }
}

fn build_statement(question: &ApiQuestion) -> String {
    let mut parts = Vec::new();

    if let Some(context) = &question.context {
        let cleaned = clean_context(context);
        if !cleaned.is_empty() {
            parts.push(cleaned);
        }
    }

    if let Some(intro) = &question.alternatives_introduction {
        let trimmed = clean_markdown_text(intro);
        if !trimmed.is_empty() {
            parts.push(trimmed);
        }
    }

    if parts.is_empty() {
        parts.push(question.title.clone());
    }

    parts.join("\n\n")
}

fn clean_context(context: &str) -> String {
    let without_images = remove_markdown_images(context);
    clean_markdown_text(&without_images.replace("\\[", "[").replace("\\]", "]"))
}

fn clean_markdown_text(input: &str) -> String {
    input.replace("**", "").replace("__", "").trim().to_string()
}

fn remove_markdown_images(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut skip_until: Option<usize> = None;

    for (index, character) in input.char_indices() {
        if let Some(end) = skip_until {
            if index < end {
                continue;
            }
            skip_until = None;
        }

        if character == '!'
            && input[index..].starts_with("![")
            && let Some(close_bracket) = input[index..].find("](")
            && let Some(close_parenthesis) = input[index + close_bracket + 2..].find(')')
        {
            skip_until = Some(index + close_bracket + 2 + close_parenthesis + 1);
            continue;
        }

        result.push(character);
    }

    result
}

fn build_options(alternatives: &[ApiAlternative]) -> (String, String, String, String, String) {
    let find = |label: &str| {
        alternatives
            .iter()
            .find(|alternative| alternative.letter.eq_ignore_ascii_case(label))
            .map(option_text)
            .unwrap_or_default()
    };

    (find("A"), find("B"), find("C"), find("D"), find("E"))
}

fn option_text(alternative: &ApiAlternative) -> String {
    alternative
        .text
        .as_deref()
        .map(clean_markdown_text)
        .filter(|text| !text.is_empty())
        .or_else(|| {
            alternative
                .file
                .as_ref()
                .map(|_| format!("Imagem da alternativa {}.", alternative.letter))
        })
        .unwrap_or_default()
}

fn alternative_letter_position(letter: &str) -> i32 {
    match letter.trim().to_uppercase().as_str() {
        "A" => 0,
        "B" => 1,
        "C" => 2,
        "D" => 3,
        "E" => 4,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_disciplines_to_areas() {
        assert_eq!(area_info("linguagens").0, "linguagens");
        assert_eq!(area_info("ciencias-humanas").0, "humanas");
        assert_eq!(area_info("ciencias-da-natureza").0, "natureza");
        assert_eq!(area_info("matematica").0, "matematica");
        assert_eq!(area_info("desconhecida").0, "linguagens");
    }

    #[test]
    fn removes_markdown_images() {
        let context = "Texto antes\n![](https://example.com/a.png)\nTexto depois";
        assert_eq!(clean_context(context), "Texto antes\n\nTexto depois");
    }

    #[test]
    fn removes_bold_markers() {
        assert_eq!(clean_context("**TEXTO I**\nConteúdo"), "TEXTO I\nConteúdo");
    }

    #[test]
    fn builds_external_id_and_code() {
        let question = ApiQuestion {
            title: "Questão 1".to_string(),
            index: 1,
            discipline: Some("linguagens".to_string()),
            language: Some("ingles".to_string()),
            year: 2023,
            context: Some("Contexto".to_string()),
            files: vec![],
            correct_alternative: "B".to_string(),
            alternatives_introduction: None,
            alternatives: vec![],
        };

        assert_eq!(external_id(&question), "enem.dev:2023:1:ingles");
        assert_eq!(question_code(&question), "ENEM-2023-001-INGLES");
    }

    #[test]
    fn parses_api_payload() {
        let payload = r#"
        {
          "metadata": { "limit": 1, "offset": 0, "total": 1, "hasMore": false },
          "questions": [{
            "title": "Questão 1 - ENEM 2023",
            "index": 1,
            "discipline": "linguagens",
            "language": "ingles",
            "year": 2023,
            "context": "Texto",
            "files": [],
            "correctAlternative": "B",
            "alternativesIntroduction": null,
            "alternatives": [
              { "letter": "A", "text": "A", "file": null, "isCorrect": false },
              { "letter": "B", "text": "B", "file": null, "isCorrect": true }
            ]
          }]
        }
        "#;
        let parsed: ApiResponse = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed.questions.len(), 1);
        assert_eq!(parsed.questions[0].correct_alternative, "B");
    }
}
