use std::time::Duration;

use clap::Parser;
use enem_backend::importer;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Parser)]
#[command(
    name = "import-enem",
    about = "Importa questões oficiais do ENEM via enem.dev para o PostgreSQL"
)]
struct Args {
    /// Ano inicial (inclusive)
    #[arg(long, default_value_t = 2009)]
    from_year: i32,

    /// Ano final (inclusive)
    #[arg(long, default_value_t = 2023)]
    to_year: i32,

    /// Limite de questões por idioma/ano (útil para testes)
    #[arg(long)]
    limit: Option<usize>,

    /// Apenas consulta a API, sem gravar no banco
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "import_enem=info,enem_backend=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    if args.from_year > args.to_year {
        return Err("--from-year não pode ser maior que --to-year".into());
    }

    let client = reqwest::Client::builder()
        .user_agent(
            "enem-aberto-importer/0.1 (+https://github.com/vinycalves/projeto-enem-backend)",
        )
        .timeout(Duration::from_secs(30))
        .build()?;

    let pool = if args.dry_run {
        None
    } else {
        let database_url =
            std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL não definida")?;
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Some(pool)
    };

    let stats = importer::run_import(
        pool.as_ref(),
        &client,
        args.from_year,
        args.to_year,
        args.limit,
    )
    .await?;

    tracing::info!(
        fetched = stats.fetched,
        upserted = stats.upserted,
        skipped = stats.skipped,
        errors = stats.errors,
        dry_run = args.dry_run,
        "importação concluída"
    );

    Ok(())
}
