use dotenvy::dotenv;
use redis::AsyncCommands;
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{env, time::Duration};
use tracing_subscriber::fmt::init as tracing_init;
use uuid::Uuid;

async fn process_job(pool: &PgPool, raw: String) -> anyhow::Result<()> {
    let value: Value = serde_json::from_str(&raw)?;
    let job_id = value
        .get("job_id")
        .and_then(|v| v.as_str())
        .and_then(|v| Uuid::parse_str(v).ok());

    let Some(job_id) = job_id else {
        println!("Invalid job payload: {}", raw);
        return Ok(());
    };

    let job_type = value
        .get("job_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    println!("Processing job {} type {}", job_id, job_type);

    sqlx::query("UPDATE jobs SET status='running', progress=25, updated_at=NOW() WHERE id=$1")
        .bind(job_id)
        .execute(pool)
        .await?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    match job_type {
        "sync_deployment_status" => {
            let deployment_id = value
                .get("payload")
                .and_then(|p| p.get("deployment_id"))
                .and_then(|v| v.as_str())
                .and_then(|v| Uuid::parse_str(v).ok());

            if let Some(deployment_id) = deployment_id {
                sqlx::query(
                    "UPDATE deployments
                     SET status='confirmed', block_number=19472810, deployed_at=NOW(), updated_at=NOW()
                     WHERE id=$1"
                )
                .bind(deployment_id)
                .execute(pool)
                .await?;
            }

            sqlx::query(
                "UPDATE jobs
                 SET status='completed', progress=100, result_json=$2, completed_at=NOW(), updated_at=NOW()
                 WHERE id=$1"
            )
            .bind(job_id)
            .bind(json!({
                "message": "Deployment status synced by local worker",
                "mock_block_number": 19472810
            }))
            .execute(pool)
            .await?;
        }
        _ => {
            sqlx::query(
                "UPDATE jobs
                 SET status='completed', progress=100, result_json=$2, completed_at=NOW(), updated_at=NOW()
                 WHERE id=$1"
            )
            .bind(job_id)
            .bind(json!({
                "message": "Generic core job completed",
                "job_type": job_type
            }))
            .execute(pool)
            .await?;
        }
    }

    println!("Job {} completed", job_id);
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_init();

    let database_url = env::var("DATABASE_URL")?;
    let redis_url = env::var("REDIS_URL")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let client = redis::Client::open(redis_url)?;
    let mut con = client.get_multiplexed_async_connection().await?;

    println!("Sentracore Core Worker running. Waiting for jobs on queue:core_jobs");

    loop {
        let result: redis::RedisResult<Option<(String, String)>> = con.brpop("queue:core_jobs", 5.0).await;

        match result {
            Ok(Some((_queue, raw))) => {
                if let Err(e) = process_job(&pool, raw).await {
                    eprintln!("Job failed: {}", e);
                }
            }
            Ok(None) => {
                println!("No job. Worker still alive...");
            }
            Err(e) => {
                eprintln!("Redis error: {}", e);
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

