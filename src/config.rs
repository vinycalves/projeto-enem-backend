#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub cors_origin: String,
    pub cookie_secure: bool,
    pub session_ttl_days: i64,
    pub pix_key: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url =
            std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL não definida".to_string())?;
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let cors_origin =
            std::env::var("CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:4200".to_string());
        let cookie_secure = std::env::var("COOKIE_SECURE")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .map_err(|_| "COOKIE_SECURE deve ser true ou false".to_string())?;
        let session_ttl_days = std::env::var("SESSION_TTL_DAYS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<i64>()
            .map_err(|_| "SESSION_TTL_DAYS deve ser um número inteiro".to_string())?;
        let pix_key = std::env::var("PIX_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Ok(Self {
            database_url,
            bind_addr,
            cors_origin,
            cookie_secure,
            session_ttl_days,
            pix_key,
        })
    }
}
