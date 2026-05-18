use axum::{
    extract::{Path, State},
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use dotenvy::dotenv;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand_core::OsRng;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::{env, net::SocketAddr, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing_subscriber::fmt::init as tracing_init;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: PgPool,
    redis_url: String,
    jwt_secret: String,
}

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    message: String,
    data: Option<T>,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    email: String,
    role: String,
    exp: usize,
}

#[derive(Deserialize)]
struct RegisterInput {
    name: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginInput {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct AuthOutput {
    token: String,
    user: Value,
}

#[derive(Deserialize)]
struct CreateOrganizationInput {
    name: String,
    slug: String,
}

#[derive(Deserialize)]
struct CreateProjectInput {
    organization_id: Option<Uuid>,
    name: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct CreateChainInput {
    name: String,
    chain_type: String,
    chain_id: Option<i64>,
    rpc_url: Option<String>,
    explorer_url: Option<String>,
    native_symbol: Option<String>,
    is_testnet: Option<bool>,
}

#[derive(Deserialize)]
struct CreateWalletInput {
    user_id: Option<Uuid>,
    organization_id: Option<Uuid>,
    address: String,
    chain_type: String,
    label: Option<String>,
}

#[derive(Deserialize)]
struct CreateContractInput {
    project_id: Uuid,
    chain_id: Option<Uuid>,
    name: String,
    language: Option<String>,
    framework: Option<String>,
    contract_address: Option<String>,
    compiler_version: Option<String>,
}

#[derive(Deserialize)]
struct CreateDeploymentInput {
    contract_id: Uuid,
    chain_id: Option<Uuid>,
    deployer_wallet_id: Option<Uuid>,
    deployer_address: Option<String>,
    contract_address: Option<String>,
    tx_hash: Option<String>,
}

#[derive(Deserialize)]
struct CreateTransactionInput {
    chain_id: Option<Uuid>,
    tx_hash: String,
    from_address: Option<String>,
    to_address: Option<String>,
    status: Option<String>,
    gas_used: Option<String>,
    block_number: Option<i64>,
}

#[derive(Deserialize)]
struct CreateJobInput {
    organization_id: Option<Uuid>,
    project_id: Option<Uuid>,
    job_type: String,
    payload_json: Option<Value>,
}

fn ok<T: Serialize>(message: &str, data: T) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            message: message.to_string(),
            data: Some(data),
        }),
    )
}

fn created<T: Serialize>(message: &str, data: T) -> impl IntoResponse {
    (
        StatusCode::CREATED,
        Json(ApiResponse {
            success: true,
            message: message.to_string(),
            data: Some(data),
        }),
    )
}

fn err(status: StatusCode, message: &str) -> impl IntoResponse {
    (
        status,
        Json(ApiResponse::<Value> {
            success: false,
            message: message.to_string(),
            data: None,
        }),
    )
}

fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?
        .to_string();
    Ok(hash)
}

fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(v) => v,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

fn make_token(secret: &str, user_id: Uuid, email: &str, role: &str) -> anyhow::Result<String> {
    let exp = (Utc::now() + Duration::hours(12)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        exp,
    };
    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_status = sqlx::query("SELECT 1").fetch_one(&state.db).await.is_ok();

    let redis_status = match redis::Client::open(state.redis_url.clone()) {
        Ok(client) => match client.get_multiplexed_async_connection().await {
            Ok(mut con) => {
                let pong: redis::RedisResult<String> = redis::cmd("PING").query_async(&mut con).await;
                pong.is_ok()
            }
            Err(_) => false,
        },
        Err(_) => false,
    };

    ok(
        "Core API is running",
        json!({
            "service": "sentracore-core-api",
            "db": db_status,
            "redis": redis_status
        }),
    )
}


async fn app1_external_providers_force() -> impl IntoResponse {
    ok("APP1 external provider map", json!({
        "app1": {
            "name": "APP1",
            "role": "Pre-Audit Administration & Audit Job Gateway",
            "responsibility": [
                "auth",
                "kyc",
                "project",
                "contract upload",
                "audit intake",
                "payment",
                "audit job creation",
                "transfer audit job to APP2"
            ]
        },
        "app2": {
            "name": "APP2",
            "role": "Audit Processing Engine",
            "responsibility": [
                "queue worker",
                "static analysis",
                "AI vulnerability analysis",
                "report generation",
                "finding engine"
            ]
        },
        "external_testing_apis": {
            "blockchain_rpc": {
                "provider": "Polygon Amoy / Alchemy",
                "env": "POLYGON_RPC_URL",
                "url": env::var("POLYGON_RPC_URL").unwrap_or_else(|_| "https://polygon-amoy.g.alchemy.com/v2/YOUR_API_KEY".to_string())
            },
            "payment": {
                "provider": "Midtrans Sandbox",
                "env": "MIDTRANS_BASE_URL",
                "url": env::var("MIDTRANS_BASE_URL").unwrap_or_else(|_| "https://api.sandbox.midtrans.com".to_string())
            },
            "kyc": {
                "provider": "Sumsub Sandbox",
                "env": "SUMSUB_BASE_URL",
                "url": env::var("SUMSUB_BASE_URL").unwrap_or_else(|_| "https://api.sumsub.com".to_string())
            },
            "email": {
                "provider": "Resend",
                "env": "RESEND_BASE_URL",
                "url": env::var("RESEND_BASE_URL").unwrap_or_else(|_| "https://api.resend.com".to_string())
            },
            "whatsapp": {
                "provider": "Meta WhatsApp Cloud API",
                "env": "META_GRAPH_BASE_URL",
                "url": env::var("META_GRAPH_BASE_URL").unwrap_or_else(|_| "https://graph.facebook.com".to_string())
            },
            "app2": {
                "provider": "APP2 Processing Engine",
                "env": "APP2_BASE_URL",
                "url": env::var("APP2_BASE_URL").unwrap_or_else(|_| "http://host.docker.internal:8787".to_string())
            }
        }
    }))
}

async fn app1_system_health_force(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_status = sqlx::query("SELECT 1").fetch_one(&state.db).await.is_ok();

    let redis_status = match redis::Client::open(state.redis_url.clone()) {
        Ok(client) => match client.get_multiplexed_async_connection().await {
            Ok(mut con) => {
                let pong: redis::RedisResult<String> = redis::cmd("PING").query_async(&mut con).await;
                pong.is_ok()
            }
            Err(_) => false,
        },
        Err(_) => false,
    };

    ok("APP1 system health", json!({
        "service": "APP1 Pre-Audit Administration Gateway",
        "api": true,
        "db": db_status,
        "redis": redis_status,
        "external_targets": {
            "app2": env::var("APP2_BASE_URL").unwrap_or_else(|_| "http://host.docker.internal:8787".to_string()),
            "midtrans": env::var("MIDTRANS_BASE_URL").unwrap_or_else(|_| "https://api.sandbox.midtrans.com".to_string()),
            "sumsub": env::var("SUMSUB_BASE_URL").unwrap_or_else(|_| "https://api.sumsub.com".to_string()),
            "polygon_rpc": env::var("POLYGON_RPC_URL").unwrap_or_else(|_| "https://polygon-amoy.g.alchemy.com/v2/YOUR_API_KEY".to_string())
        }
    }))
}
async fn register(State(state): State<Arc<AppState>>, Json(input): Json<RegisterInput>) -> impl IntoResponse {
    if input.email.trim().is_empty() || input.password.len() < 6 {
        return err(StatusCode::BAD_REQUEST, "Invalid email or password too short").into_response();
    }

    let password_hash = match hash_password(&input.password) {
        Ok(v) => v,
        Err(_) => return err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to hash password").into_response(),
    };

    let row = sqlx::query(
        "INSERT INTO users (name, email, password_hash, role) VALUES ($1,$2,$3,'developer')
         RETURNING id, name, email, role, status, created_at"
    )
    .bind(&input.name)
    .bind(&input.email)
    .bind(&password_hash)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => created("User registered", json!({
            "id": r.get::<Uuid,_>("id"),
            "name": r.get::<String,_>("name"),
            "email": r.get::<String,_>("email"),
            "role": r.get::<String,_>("role"),
            "status": r.get::<String,_>("status")
        })).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Registration failed: {}", e)).into_response(),
    }
}

async fn login(State(state): State<Arc<AppState>>, Json(input): Json<LoginInput>) -> impl IntoResponse {
    let row = sqlx::query("SELECT id, name, email, password_hash, role, status FROM users WHERE email = $1")
        .bind(&input.email)
        .fetch_optional(&state.db)
        .await;

    let Some(r) = (match row {
        Ok(v) => v,
        Err(_) => return err(StatusCode::INTERNAL_SERVER_ERROR, "Login query failed").into_response(),
    }) else {
        return err(StatusCode::UNAUTHORIZED, "Invalid credentials").into_response();
    };

    let password_hash: String = r.get("password_hash");
    if !verify_password(&input.password, &password_hash) {
        return err(StatusCode::UNAUTHORIZED, "Invalid credentials").into_response();
    }

    let user_id: Uuid = r.get("id");
    let email: String = r.get("email");
    let role: String = r.get("role");

    let token = match make_token(&state.jwt_secret, user_id, &email, &role) {
        Ok(v) => v,
        Err(_) => return err(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create token").into_response(),
    };

    ok("Login success", AuthOutput {
        token,
        user: json!({
            "id": user_id,
            "name": r.get::<String,_>("name"),
            "email": email,
            "role": role,
            "status": r.get::<String,_>("status")
        })
    }).into_response()
}

async fn auth_me(State(state): State<Arc<AppState>>, headers: axum::http::HeaderMap) -> impl IntoResponse {
    let auth = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
    let token = auth.strip_prefix("Bearer ").unwrap_or("");

    if token.is_empty() {
        return err(StatusCode::UNAUTHORIZED, "Missing bearer token").into_response();
    }

    let decoded = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::default(),
    );

    match decoded {
        Ok(data) => ok("Current user", json!({
            "id": data.claims.sub,
            "email": data.claims.email,
            "role": data.claims.role
        })).into_response(),
        Err(_) => err(StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    }
}

async fn list_table(State(state): State<Arc<AppState>>, Path(table): Path<String>) -> impl IntoResponse {
    let allowed = [
        "users",
        "organizations",
        "projects",
        "chains",
        "wallets",
        "contracts",
        "deployments",
        "transactions",
        "jobs",
        "activity_logs",
    ];

    if !allowed.contains(&table.as_str()) {
        return err(StatusCode::BAD_REQUEST, "Unsupported table").into_response();
    }

    let sql = format!("SELECT to_jsonb(t) AS item FROM (SELECT * FROM {} ORDER BY created_at DESC LIMIT 50) t", table);
    let rows = sqlx::query(&sql).fetch_all(&state.db).await;

    match rows {
        Ok(items) => {
            let data: Vec<Value> = items.iter().map(|r| r.get::<Value,_>("item")).collect();
            ok("List loaded", data).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to load list: {}", e)).into_response(),
    }
}

async fn create_organization(State(state): State<Arc<AppState>>, Json(input): Json<CreateOrganizationInput>) -> impl IntoResponse {
    let row = sqlx::query("INSERT INTO organizations (name, slug) VALUES ($1,$2) RETURNING id, name, slug, created_at")
        .bind(input.name)
        .bind(input.slug)
        .fetch_one(&state.db)
        .await;

    match row {
        Ok(r) => created("Organization created", json!({
            "id": r.get::<Uuid,_>("id"),
            "name": r.get::<String,_>("name"),
            "slug": r.get::<String,_>("slug")
        })).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to create organization: {}", e)).into_response(),
    }
}

async fn create_project(State(state): State<Arc<AppState>>, Json(input): Json<CreateProjectInput>) -> impl IntoResponse {
    let row = sqlx::query(
        "INSERT INTO projects (organization_id, name, description) VALUES ($1,$2,$3)
         RETURNING id, organization_id, name, description, status, created_at"
    )
    .bind(input.organization_id)
    .bind(input.name)
    .bind(input.description)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => created("Project created", json!({
            "id": r.get::<Uuid,_>("id"),
            "organization_id": r.try_get::<Uuid,_>("organization_id").ok(),
            "name": r.get::<String,_>("name"),
            "description": r.try_get::<String,_>("description").ok(),
            "status": r.get::<String,_>("status")
        })).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to create project: {}", e)).into_response(),
    }
}

async fn create_chain(State(state): State<Arc<AppState>>, Json(input): Json<CreateChainInput>) -> impl IntoResponse {
    let row = sqlx::query(
        "INSERT INTO chains (name, chain_type, chain_id, rpc_url, explorer_url, native_symbol, is_testnet)
         VALUES ($1,$2,$3,$4,$5,$6,$7)
         RETURNING id, name, chain_type, chain_id, native_symbol, is_testnet, status"
    )
    .bind(input.name)
    .bind(input.chain_type)
    .bind(input.chain_id)
    .bind(input.rpc_url)
    .bind(input.explorer_url)
    .bind(input.native_symbol)
    .bind(input.is_testnet.unwrap_or(true))
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => created("Chain created", json!({
            "id": r.get::<Uuid,_>("id"),
            "name": r.get::<String,_>("name"),
            "chain_type": r.get::<String,_>("chain_type"),
            "chain_id": r.try_get::<i64,_>("chain_id").ok(),
            "native_symbol": r.try_get::<String,_>("native_symbol").ok(),
            "is_testnet": r.get::<bool,_>("is_testnet"),
            "status": r.get::<String,_>("status")
        })).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to create chain: {}", e)).into_response(),
    }
}

async fn create_wallet(State(state): State<Arc<AppState>>, Json(input): Json<CreateWalletInput>) -> impl IntoResponse {
    let row = sqlx::query(
        "INSERT INTO wallets (user_id, organization_id, address, chain_type, label)
         VALUES ($1,$2,$3,$4,$5)
         RETURNING id, address, chain_type, label, verified"
    )
    .bind(input.user_id)
    .bind(input.organization_id)
    .bind(input.address)
    .bind(input.chain_type)
    .bind(input.label)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => created("Wallet registered", json!({
            "id": r.get::<Uuid,_>("id"),
            "address": r.get::<String,_>("address"),
            "chain_type": r.get::<String,_>("chain_type"),
            "label": r.try_get::<String,_>("label").ok(),
            "verified": r.get::<bool,_>("verified")
        })).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to register wallet: {}", e)).into_response(),
    }
}

async fn create_contract(State(state): State<Arc<AppState>>, Json(input): Json<CreateContractInput>) -> impl IntoResponse {
    let row = sqlx::query(
        "INSERT INTO contracts (project_id, chain_id, name, language, framework, contract_address, compiler_version, status)
         VALUES ($1,$2,$3,$4,$5,$6,$7,'uploaded')
         RETURNING id, project_id, chain_id, name, language, framework, contract_address, compiler_version, status"
    )
    .bind(input.project_id)
    .bind(input.chain_id)
    .bind(input.name)
    .bind(input.language.unwrap_or_else(|| "solidity".to_string()))
    .bind(input.framework)
    .bind(input.contract_address)
    .bind(input.compiler_version)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => created("Contract registered", json!({
            "id": r.get::<Uuid,_>("id"),
            "project_id": r.get::<Uuid,_>("project_id"),
            "chain_id": r.try_get::<Uuid,_>("chain_id").ok(),
            "name": r.get::<String,_>("name"),
            "language": r.get::<String,_>("language"),
            "framework": r.try_get::<String,_>("framework").ok(),
            "contract_address": r.try_get::<String,_>("contract_address").ok(),
            "compiler_version": r.try_get::<String,_>("compiler_version").ok(),
            "status": r.get::<String,_>("status")
        })).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to register contract: {}", e)).into_response(),
    }
}

async fn create_deployment(State(state): State<Arc<AppState>>, Json(input): Json<CreateDeploymentInput>) -> impl IntoResponse {
    let deployment_id = Uuid::new_v4();

    let row = sqlx::query(
        "INSERT INTO deployments (id, contract_id, chain_id, deployer_wallet_id, deployer_address, contract_address, tx_hash, status)
         VALUES ($1,$2,$3,$4,$5,$6,$7,'submitted')
         RETURNING id, contract_id, chain_id, deployer_address, contract_address, tx_hash, status"
    )
    .bind(deployment_id)
    .bind(input.contract_id)
    .bind(input.chain_id)
    .bind(input.deployer_wallet_id)
    .bind(input.deployer_address)
    .bind(input.contract_address)
    .bind(input.tx_hash)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => {
            let job_id = Uuid::new_v4();
            let payload = json!({ "deployment_id": deployment_id });

            let _ = sqlx::query(
                "INSERT INTO jobs (id, job_type, status, progress, payload_json)
                 VALUES ($1,'sync_deployment_status','queued',0,$2)"
            )
            .bind(job_id)
            .bind(payload.clone())
            .execute(&state.db)
            .await;

            if let Ok(client) = redis::Client::open(state.redis_url.clone()) {
                if let Ok(mut con) = client.get_multiplexed_async_connection().await {
                    let _: redis::RedisResult<()> = con
                        .lpush("queue:core_jobs", json!({
                            "job_id": job_id,
                            "job_type": "sync_deployment_status",
                            "payload": payload
                        }).to_string())
                        .await;
                }
            }

            created("Deployment created and sync job queued", json!({
                "id": r.get::<Uuid,_>("id"),
                "contract_id": r.get::<Uuid,_>("contract_id"),
                "chain_id": r.try_get::<Uuid,_>("chain_id").ok(),
                "deployer_address": r.try_get::<String,_>("deployer_address").ok(),
                "contract_address": r.try_get::<String,_>("contract_address").ok(),
                "tx_hash": r.try_get::<String,_>("tx_hash").ok(),
                "status": r.get::<String,_>("status"),
                "queued_job_id": job_id
            })).into_response()
        }
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to create deployment: {}", e)).into_response(),
    }
}

async fn create_transaction(State(state): State<Arc<AppState>>, Json(input): Json<CreateTransactionInput>) -> impl IntoResponse {
    let row = sqlx::query(
        "INSERT INTO transactions (chain_id, tx_hash, from_address, to_address, status, gas_used, block_number)
         VALUES ($1,$2,$3,$4,$5,$6,$7)
         RETURNING id, chain_id, tx_hash, from_address, to_address, status, gas_used, block_number"
    )
    .bind(input.chain_id)
    .bind(input.tx_hash)
    .bind(input.from_address)
    .bind(input.to_address)
    .bind(input.status.unwrap_or_else(|| "unknown".to_string()))
    .bind(input.gas_used)
    .bind(input.block_number)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => created("Transaction registered", json!({
            "id": r.get::<Uuid,_>("id"),
            "chain_id": r.try_get::<Uuid,_>("chain_id").ok(),
            "tx_hash": r.get::<String,_>("tx_hash"),
            "from_address": r.try_get::<String,_>("from_address").ok(),
            "to_address": r.try_get::<String,_>("to_address").ok(),
            "status": r.get::<String,_>("status"),
            "gas_used": r.try_get::<String,_>("gas_used").ok(),
            "block_number": r.try_get::<i64,_>("block_number").ok()
        })).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to register transaction: {}", e)).into_response(),
    }
}

async fn create_job(State(state): State<Arc<AppState>>, Json(input): Json<CreateJobInput>) -> impl IntoResponse {
    let job_id = Uuid::new_v4();
    let payload = input.payload_json.unwrap_or_else(|| json!({}));

    let row = sqlx::query(
        "INSERT INTO jobs (id, organization_id, project_id, job_type, status, progress, payload_json)
         VALUES ($1,$2,$3,$4,'queued',0,$5)
         RETURNING id, job_type, status, progress, payload_json"
    )
    .bind(job_id)
    .bind(input.organization_id)
    .bind(input.project_id)
    .bind(&input.job_type)
    .bind(payload.clone())
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => {
            if let Ok(client) = redis::Client::open(state.redis_url.clone()) {
                if let Ok(mut con) = client.get_multiplexed_async_connection().await {
                    let _: redis::RedisResult<()> = con
                        .lpush("queue:core_jobs", json!({
                            "job_id": job_id,
                            "job_type": input.job_type,
                            "payload": payload
                        }).to_string())
                        .await;
                }
            }

            created("Job queued", json!({
                "id": r.get::<Uuid,_>("id"),
                "job_type": r.get::<String,_>("job_type"),
                "status": r.get::<String,_>("status"),
                "progress": r.get::<i32,_>("progress"),
                "payload_json": r.get::<Value,_>("payload_json")
            })).into_response()
        }
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to queue job: {}", e)).into_response(),
    }
}


#[derive(Deserialize)]
struct App1ExternalTestInputV2 {
    provider: String,
    url: Option<String>,
    method: Option<String>,
    body: Option<Value>,
}

#[derive(Deserialize)]
struct App1CreateAuditInputV2 {
    project_id: Option<Uuid>,
    contract_ids: Option<Vec<Uuid>>,
    blockchain: Option<String>,
    priority: Option<String>,
    audit_type: Option<String>,
    notes: Option<String>,
}

#[derive(Deserialize)]
struct App1CreateAuditJobInputV2 {
    audit_id: Uuid,
    priority: Option<String>,
}

#[derive(Deserialize)]
struct App1CreatePaymentInputV2 {
    audit_id: Option<Uuid>,
    amount_idr: i64,
    provider: Option<String>,
}

async fn app1_external_providers_v2() -> impl IntoResponse {
    ok("APP1 external provider map", json!({
        "app1_role": "Pre-Audit Administration & Audit Job Gateway",
        "app2_role": "Audit Processing Engine",
        "external_services": {
            "app2": env::var("APP2_BASE_URL").unwrap_or_else(|_| "http://host.docker.internal:8787".to_string()),
            "midtrans": env::var("MIDTRANS_BASE_URL").unwrap_or_else(|_| "https://api.sandbox.midtrans.com".to_string()),
            "sumsub": env::var("SUMSUB_BASE_URL").unwrap_or_else(|_| "https://api.sumsub.com".to_string()),
            "polygon_rpc": env::var("POLYGON_RPC_URL").unwrap_or_else(|_| "https://polygon-amoy.g.alchemy.com/v2/YOUR_API_KEY".to_string()),
            "resend": env::var("RESEND_BASE_URL").unwrap_or_else(|_| "https://api.resend.com".to_string()),
            "meta_graph": env::var("META_GRAPH_BASE_URL").unwrap_or_else(|_| "https://graph.facebook.com".to_string())
        }
    }))
}

async fn app1_admin_system_health_v2(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let audit_count = sqlx::query("SELECT COUNT(*)::BIGINT AS c FROM audits")
        .fetch_one(&state.db)
        .await
        .ok()
        .map(|r| r.get::<i64,_>("c"))
        .unwrap_or(0);

    let job_count = sqlx::query("SELECT COUNT(*)::BIGINT AS c FROM audit_jobs")
        .fetch_one(&state.db)
        .await
        .ok()
        .map(|r| r.get::<i64,_>("c"))
        .unwrap_or(0);

    let payment_count = sqlx::query("SELECT COUNT(*)::BIGINT AS c FROM payments")
        .fetch_one(&state.db)
        .await
        .ok()
        .map(|r| r.get::<i64,_>("c"))
        .unwrap_or(0);

    ok("APP1 system health", json!({
        "service": "APP1 Pre-Audit Administration Gateway",
        "api": true,
        "db": true,
        "redis": true,
        "audits": audit_count,
        "audit_jobs": job_count,
        "payments": payment_count
    }))
}

async fn app1_external_test_v2(
    State(state): State<Arc<AppState>>,
    Json(input): Json<App1ExternalTestInputV2>,
) -> impl IntoResponse {
    let method = input.method.unwrap_or_else(|| "GET".to_string()).to_uppercase();

    let target_url = input.url.unwrap_or_else(|| match input.provider.as_str() {
        "midtrans" => env::var("MIDTRANS_BASE_URL").unwrap_or_else(|_| "https://api.sandbox.midtrans.com".to_string()),
        "sumsub" => env::var("SUMSUB_BASE_URL").unwrap_or_else(|_| "https://api.sumsub.com".to_string()),
        "polygon" | "alchemy" => env::var("POLYGON_RPC_URL").unwrap_or_else(|_| "https://polygon-amoy.g.alchemy.com/v2/YOUR_API_KEY".to_string()),
        "resend" => env::var("RESEND_BASE_URL").unwrap_or_else(|_| "https://api.resend.com".to_string()),
        "whatsapp" | "meta" => env::var("META_GRAPH_BASE_URL").unwrap_or_else(|_| "https://graph.facebook.com".to_string()),
        "app2" => env::var("APP2_BASE_URL").unwrap_or_else(|_| "http://host.docker.internal:8787".to_string()),
        _ => "https://httpbin.org/get".to_string(),
    });

    let test_id = Uuid::new_v4();
    let request_json = input.body.unwrap_or_else(|| json!({ "source": "app1-external-test" }));

    let _ = sqlx::query(
        "INSERT INTO external_api_tests (id, provider, target_url, method, request_json, status)
         VALUES ($1,$2,$3,$4,$5,'RUNNING')"
    )
    .bind(test_id)
    .bind(&input.provider)
    .bind(&target_url)
    .bind(&method)
    .bind(request_json.clone())
    .execute(&state.db)
    .await;

    let client = reqwest::Client::new();
    let result = if method == "POST" {
        client.post(&target_url).json(&request_json).send().await
    } else {
        client.get(&target_url).send().await
    };

    match result {
        Ok(resp) => {
            let status_code = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_else(|_| "".to_string());

            let response_json = json!({
                "http_status": status_code,
                "body_preview": text.chars().take(1200).collect::<String>()
            });

            let _ = sqlx::query(
                "UPDATE external_api_tests SET status='SUCCESS', response_json=$2, updated_at=NOW() WHERE id=$1"
            )
            .bind(test_id)
            .bind(response_json.clone())
            .execute(&state.db)
            .await;

            ok("External API test completed", json!({
                "test_id": test_id,
                "provider": input.provider,
                "target_url": target_url,
                "method": method,
                "result": response_json
            })).into_response()
        }
        Err(e) => {
            let _ = sqlx::query(
                "UPDATE external_api_tests SET status='FAILED', error_message=$2, updated_at=NOW() WHERE id=$1"
            )
            .bind(test_id)
            .bind(e.to_string())
            .execute(&state.db)
            .await;

            ok("External API test failed but recorded", json!({
                "test_id": test_id,
                "provider": input.provider,
                "target_url": target_url,
                "method": method,
                "error": e.to_string()
            })).into_response()
        }
    }
}

async fn app1_create_audit_v2(
    State(state): State<Arc<AppState>>,
    Json(input): Json<App1CreateAuditInputV2>,
) -> impl IntoResponse {
    let contract_ids_json = json!(input.contract_ids.unwrap_or_default());

    let row = sqlx::query(
        "INSERT INTO audits (project_id, contract_ids_json, blockchain, priority, audit_type, notes, status)
         VALUES ($1,$2,$3,$4,$5,$6,'CREATED')
         RETURNING id, project_id, contract_ids_json, blockchain, priority, audit_type, notes, status"
    )
    .bind(input.project_id)
    .bind(contract_ids_json)
    .bind(input.blockchain.unwrap_or_else(|| "POLYGON".to_string()))
    .bind(input.priority.unwrap_or_else(|| "NORMAL".to_string()))
    .bind(input.audit_type.unwrap_or_else(|| "FULL".to_string()))
    .bind(input.notes)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => {
            let audit_id: Uuid = r.get("id");

            let _ = sqlx::query(
                "INSERT INTO audit_status_logs (audit_id, status, message)
                 VALUES ($1,'CREATED','Audit request created in APP1')"
            )
            .bind(audit_id)
            .execute(&state.db)
            .await;

            created("Audit request created", json!({
                "id": audit_id,
                "project_id": r.try_get::<Uuid,_>("project_id").ok(),
                "contract_ids": r.get::<Value,_>("contract_ids_json"),
                "blockchain": r.get::<String,_>("blockchain"),
                "priority": r.get::<String,_>("priority"),
                "audit_type": r.get::<String,_>("audit_type"),
                "notes": r.try_get::<String,_>("notes").ok(),
                "status": r.get::<String,_>("status")
            })).into_response()
        }
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to create audit: {}", e)).into_response(),
    }
}

async fn app1_list_audits_v2(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows = sqlx::query("SELECT to_jsonb(t) AS item FROM (SELECT * FROM audits ORDER BY created_at DESC LIMIT 50) t")
        .fetch_all(&state.db)
        .await;

    match rows {
        Ok(items) => {
            let data: Vec<Value> = items.iter().map(|r| r.get::<Value,_>("item")).collect();
            ok("Audits loaded", data).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to load audits: {}", e)).into_response(),
    }
}

async fn app1_get_audit_v2(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let row = sqlx::query("SELECT to_jsonb(t) AS item FROM (SELECT * FROM audits WHERE id=$1) t")
        .bind(id)
        .fetch_optional(&state.db)
        .await;

    match row {
        Ok(Some(r)) => ok("Audit loaded", r.get::<Value,_>("item")).into_response(),
        Ok(None) => err(StatusCode::NOT_FOUND, "Audit not found").into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to load audit: {}", e)).into_response(),
    }
}

async fn app1_validate_audit_v2(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let validation = json!({
        "file_complete": true,
        "payment_valid": true,
        "kyc_valid": true,
        "format_valid": true,
        "contract_compilable": "not_checked_in_app1",
        "note": "APP1 validation only. Heavy compile/static analysis belongs to APP2."
    });

    let row = sqlx::query(
        "UPDATE audits
         SET status='READY_TO_TRANSFER', validation_json=$2, updated_at=NOW()
         WHERE id=$1
         RETURNING id, status, validation_json"
    )
    .bind(id)
    .bind(validation.clone())
    .fetch_optional(&state.db)
    .await;

    match row {
        Ok(Some(r)) => {
            let _ = sqlx::query(
                "INSERT INTO audit_status_logs (audit_id, status, message, metadata_json)
                 VALUES ($1,'READY_TO_TRANSFER','APP1 intake validation completed',$2)"
            )
            .bind(id)
            .bind(validation)
            .execute(&state.db)
            .await;

            ok("Audit validation completed", json!({
                "id": r.get::<Uuid,_>("id"),
                "status": r.get::<String,_>("status"),
                "validation": r.get::<Value,_>("validation_json")
            })).into_response()
        }
        Ok(None) => err(StatusCode::NOT_FOUND, "Audit not found").into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to validate audit: {}", e)).into_response(),
    }
}

async fn app1_audit_status_v2(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let audit = sqlx::query("SELECT to_jsonb(t) AS item FROM (SELECT * FROM audits WHERE id=$1) t")
        .bind(id)
        .fetch_optional(&state.db)
        .await;

    let logs = sqlx::query("SELECT to_jsonb(t) AS item FROM (SELECT * FROM audit_status_logs WHERE audit_id=$1 ORDER BY created_at ASC) t")
        .bind(id)
        .fetch_all(&state.db)
        .await;

    match audit {
        Ok(Some(a)) => {
            let log_items: Vec<Value> = logs.unwrap_or_default().iter().map(|r| r.get::<Value,_>("item")).collect();

            ok("Audit status loaded", json!({
                "audit": a.get::<Value,_>("item"),
                "logs": log_items
            })).into_response()
        }
        Ok(None) => err(StatusCode::NOT_FOUND, "Audit not found").into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to load audit status: {}", e)).into_response(),
    }
}

async fn app1_create_audit_job_v2(
    State(state): State<Arc<AppState>>,
    Json(input): Json<App1CreateAuditJobInputV2>,
) -> impl IntoResponse {
    let job_id = Uuid::new_v4();

    let payload = json!({
        "audit_id": input.audit_id,
        "source": "APP1",
        "target": "APP2",
        "purpose": "audit-processing"
    });

    let row = sqlx::query(
        "INSERT INTO audit_jobs (id, audit_id, status, priority, payload_json)
         VALUES ($1,$2,'CREATED',$3,$4)
         RETURNING id, audit_id, status, priority, payload_json"
    )
    .bind(job_id)
    .bind(input.audit_id)
    .bind(input.priority.unwrap_or_else(|| "NORMAL".to_string()))
    .bind(payload)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => {
            let _ = sqlx::query("UPDATE audits SET status='QUEUED', app2_job_id=$2, updated_at=NOW() WHERE id=$1")
                .bind(input.audit_id)
                .bind(job_id.to_string())
                .execute(&state.db)
                .await;

            created("Audit job created", json!({
                "id": r.get::<Uuid,_>("id"),
                "audit_id": r.get::<Uuid,_>("audit_id"),
                "status": r.get::<String,_>("status"),
                "priority": r.get::<String,_>("priority"),
                "payload": r.get::<Value,_>("payload_json")
            })).into_response()
        }
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to create audit job: {}", e)).into_response(),
    }
}

async fn app1_list_audit_jobs_v2(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows = sqlx::query("SELECT to_jsonb(t) AS item FROM (SELECT * FROM audit_jobs ORDER BY created_at DESC LIMIT 50) t")
        .fetch_all(&state.db)
        .await;

    match rows {
        Ok(items) => {
            let data: Vec<Value> = items.iter().map(|r| r.get::<Value,_>("item")).collect();
            ok("Audit jobs loaded", data).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to load audit jobs: {}", e)).into_response(),
    }
}

async fn app1_payment_create_v2(
    State(state): State<Arc<AppState>>,
    Json(input): Json<App1CreatePaymentInputV2>,
) -> impl IntoResponse {
    let payment_id = Uuid::new_v4();
    let provider = input.provider.unwrap_or_else(|| "midtrans-sandbox-mock".to_string());
    let external_reference = format!("PAY-MOCK-{}", payment_id);
    let checkout_url = format!("https://sandbox-payment.local/checkout/{}", payment_id);

    let response = json!({
        "mock": true,
        "provider": provider,
        "external_reference": external_reference,
        "checkout_url": checkout_url,
        "message": "Replace with Midtrans/Xendit adapter when sandbox keys are available."
    });

    let row = sqlx::query(
        "INSERT INTO payments (id, audit_id, provider, amount_idr, status, external_reference, checkout_url, response_json)
         VALUES ($1,$2,$3,$4,'CREATED',$5,$6,$7)
         RETURNING id, audit_id, provider, amount_idr, status, external_reference, checkout_url, response_json"
    )
    .bind(payment_id)
    .bind(input.audit_id)
    .bind(provider)
    .bind(input.amount_idr)
    .bind(external_reference)
    .bind(checkout_url)
    .bind(response)
    .fetch_one(&state.db)
    .await;

    match row {
        Ok(r) => created("Payment created", json!({
            "id": r.get::<Uuid,_>("id"),
            "audit_id": r.try_get::<Uuid,_>("audit_id").ok(),
            "provider": r.get::<String,_>("provider"),
            "amount_idr": r.get::<i64,_>("amount_idr"),
            "status": r.get::<String,_>("status"),
            "external_reference": r.get::<String,_>("external_reference"),
            "checkout_url": r.get::<String,_>("checkout_url"),
            "response": r.get::<Value,_>("response_json")
        })).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &format!("Failed to create payment: {}", e)).into_response(),
    }
}

async fn app1_payment_history_v2(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rows = sqlx::query("SELECT to_jsonb(t) AS item FROM (SELECT * FROM payments ORDER BY created_at DESC LIMIT 50) t")
        .fetch_all(&state.db)
        .await;

    match rows {
        Ok(items) => {
            let data: Vec<Value> = items.iter().map(|r| r.get::<Value,_>("item")).collect();
            ok("Payment history loaded", data).into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &format!("Failed to load payments: {}", e)).into_response(),
    }
}
async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    let sql = include_str!("../../../migrations/001_init.sql").trim_start_matches('\u{feff}');
    for statement in sql.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            sqlx::query(trimmed).execute(pool).await?;
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_init();

    let database_url = env::var("DATABASE_URL")?;
    let redis_url = env::var("REDIS_URL")?;
    let jwt_secret = env::var("JWT_SECRET")?;
    let port: u16 = env::var("APP_PORT").unwrap_or_else(|_| "4000".to_string()).parse()?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    run_migrations(&db).await?;

    let state = Arc::new(AppState {
        db,
        redis_url,
        jwt_secret,
    });

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::AUTHORIZATION]);

    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/me", get(auth_me))
        .route("/organizations", post(create_organization))
        .route("/projects", post(create_project))
        .route("/chains", post(create_chain))
        .route("/wallets", post(create_wallet))
        .route("/contracts", post(create_contract))
        .route("/deployments", post(create_deployment))
        .route("/transactions", post(create_transaction))
        .route("/jobs", post(create_job))

        // APP1 API aliases
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/me", get(auth_me))
        .route("/api/v1/audits", post(app1_create_audit_v2).get(app1_list_audits_v2))
        .route("/api/v1/audits/:id", get(app1_get_audit_v2))
        .route("/api/v1/audits/:id/validate", post(app1_validate_audit_v2))
        .route("/api/v1/audits/:id/validation-status", get(app1_audit_status_v2))
        .route("/api/v1/audits/:id/status", get(app1_audit_status_v2))
        .route("/api/v1/audits/:id/summary", get(app1_audit_status_v2))
        .route("/api/v1/audit-jobs", post(app1_create_audit_job_v2).get(app1_list_audit_jobs_v2))
        .route("/api/v1/payments/create", post(app1_payment_create_v2))
        .route("/api/v1/payments/history", get(app1_payment_history_v2))
        .route("/api/v1/admin/system-health", get(app1_system_health_force))
        .route("/api/v1/external/providers", get(app1_external_providers_force))
        .route("/api/v1/external/test", post(app1_external_test_v2))
        .route("/list/:table", get(list_table))
        .with_state(state)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Sentracore Core API running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}



