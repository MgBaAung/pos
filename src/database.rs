use crate::config::Settings;
use crate::error::{AppError, AppResult};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

/// Database connection pool
#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn new(settings: &Settings) -> AppResult<Self> {
        tracing::info!("Connecting to database...");

        let pool = PgPoolOptions::new()
            .max_connections(settings.database.max_connections)
            .min_connections(settings.database.min_connections)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .max_lifetime(Duration::from_secs(1800))
            .connect(&settings.database.url)
            .await
            .map_err(AppError::Database)?;

        tracing::info!("Database connection pool created successfully");
        tracing::debug!(
            "Pool configuration: max_connections={}, min_connections={}",
            settings.database.max_connections,
            settings.database.min_connections
        );

        Ok(Database { pool })
    }

    /// Run database migrations
    pub async fn migrate(&self) -> AppResult<()> {
        tracing::info!("Running database migrations...");

        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Migration failed: {}", e);
                AppError::Database(e.into())
            })?;

        tracing::info!("Database migrations completed successfully");
        Ok(())
    }

    /// Get a connection from the pool
    pub async fn acquire(&self) -> AppResult<sqlx::pool::PoolConnection<sqlx::Postgres>> {
        self.pool.acquire().await.map_err(|e| AppError::Database(e))
    }

    /// Begin a transaction
    pub async fn begin(&self) -> AppResult<sqlx::Transaction<'_, sqlx::Postgres>> {
        self.pool.begin().await.map_err(|e| AppError::Database(e))
    }

    /// Health check for database connection
    pub async fn health_check(&self) -> AppResult<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e))?;

        Ok(())
    }

    /// Close the database connection pool
    pub async fn close(self) {
        tracing::info!("Closing database connection pool...");
        self.pool.close().await;
        tracing::info!("Database connection pool closed");
    }
}

/// Extension trait for Axum to add database to request state
pub trait DatabaseExt {
    fn database(&self) -> &Database;
}

impl DatabaseExt for axum::extract::State<Database> {
    fn database(&self) -> &Database {
        &self.0
    }
}
