use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::{env, time::Duration};

pub async fn establish_connection() -> Result<PgPool, sqlx::Error> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://admin:adminpass@localhost:5432/taskmanager".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await?;

    log::info!("Connected to database");
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Create tasks table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            title VARCHAR(255) NOT NULL,
            description TEXT DEFAULT '',
            status VARCHAR(20) NOT NULL DEFAULT 'todo'
                CHECK (status IN ('todo', 'in_progress', 'done')),
            priority VARCHAR(10) NOT NULL DEFAULT 'medium'
                CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
            due_date TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            completed_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority);
        "#,
    )
    .execute(pool)
    .await?;

    // Create the updated_at trigger
    sqlx::query(
        r#"
        CREATE OR REPLACE FUNCTION update_updated_at_column()
        RETURNS TRIGGER AS $$
        BEGIN
            NEW.updated_at = NOW();
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
        "#,
    )
    .execute(pool)
    .await?;

    // Apply trigger
    sqlx::query(
        r#"
        DROP TRIGGER IF EXISTS set_updated_at ON tasks;
        CREATE TRIGGER set_updated_at
            BEFORE UPDATE ON tasks
            FOR EACH ROW
            EXECUTE FUNCTION update_updated_at_column();
        "#,
    )
    .execute(pool)
    .await?;

    log::info!("Database migration completed");
    Ok(())
}
