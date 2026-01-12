use dotenv::dotenv;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::error::Error;
use std::str::FromStr;
use tracing::info;

pub async fn db_connection() -> Result<SqlitePool, Box<dyn Error>> {
    dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        tracing::warn!("DATABASE_URL not set, using default SQLite file");
        "sqlite:vrp_database.sqlite".to_string()
    });

    let options = SqliteConnectOptions::from_str(&database_url)?.create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    info!("Connected to SQLite database at {database_url}");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS api_tokens (
            service TEXT PRIMARY KEY,
            token TEXT NOT NULL,
            expiry INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    return Ok(pool);
}
