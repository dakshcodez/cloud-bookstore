use std::{env, hint::black_box, sync::Arc, time::{Duration, Instant}};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use tower_http::{services::ServeDir, trace::TraceLayer};

const SCHEMA: &str = include_str!("../schema.sql");

#[derive(Clone)]
struct AppState {
    db: PgPool,
    info: Arc<InstanceInfo>,
}

#[derive(Serialize)]
struct InstanceInfo {
    instance_id: String,
    availability_zone: String,
}

#[derive(Serialize, FromRow)]
struct Book {
    id: i32,
    title: String,
    author: String,
    price_cents: i32,
}

#[derive(Serialize, FromRow)]
struct Order {
    id: i32,
    customer_name: String,
    book_title: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
struct NewOrder {
    book_id: i32,
    customer_name: String,
}

type ApiError = (StatusCode, String);

fn internal(e: sqlx::Error) -> ApiError {
    tracing::error!("db error: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "database error".into())
}

async fn health(State(s): State<AppState>) -> StatusCode {
    match sqlx::query("SELECT 1").execute(&s.db).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn info(State(s): State<AppState>) -> Json<Arc<InstanceInfo>> {
    Json(s.info.clone())
}

async fn list_books(State(s): State<AppState>) -> Result<Json<Vec<Book>>, ApiError> {
    sqlx::query_as::<_, Book>("SELECT id, title, author, price_cents FROM books ORDER BY id")
        .fetch_all(&s.db)
        .await
        .map(Json)
        .map_err(internal)
}

async fn list_orders(State(s): State<AppState>) -> Result<Json<Vec<Order>>, ApiError> {
    sqlx::query_as::<_, Order>(
        "SELECT o.id, o.customer_name, b.title AS book_title, o.created_at
         FROM orders o JOIN books b ON b.id = o.book_id
         ORDER BY o.id DESC LIMIT 50",
    )
    .fetch_all(&s.db)
    .await
    .map(Json)
    .map_err(internal)
}

async fn create_order(
    State(s): State<AppState>,
    Json(o): Json<NewOrder>,
) -> Result<(StatusCode, Json<Order>), ApiError> {
    let name = o.customer_name.trim();
    if name.is_empty() || name.len() > 100 {
        return Err((StatusCode::BAD_REQUEST, "customer_name must be 1-100 characters".into()));
    }
    let row = sqlx::query_as::<_, Order>(
        "WITH ins AS (
             INSERT INTO orders (book_id, customer_name)
             SELECT id, $2 FROM books WHERE id = $1
             RETURNING id, book_id, customer_name, created_at
         )
         SELECT ins.id, ins.customer_name, b.title AS book_title, ins.created_at
         FROM ins JOIN books b ON b.id = ins.book_id",
    )
    .bind(o.book_id)
    .bind(name)
    .fetch_optional(&s.db)
    .await
    .map_err(internal)?;
    match row {
        Some(order) => Ok((StatusCode::CREATED, Json(order))),
        None => Err((StatusCode::BAD_REQUEST, "unknown book_id".into())),
    }
}

#[derive(Deserialize)]
struct StressParams {
    ms: Option<u64>,
}

/// Burns CPU for `ms` milliseconds (capped at 1000). Used to drive Auto Scaling in load tests.
async fn stress(Query(p): Query<StressParams>) -> Json<serde_json::Value> {
    let ms = p.ms.unwrap_or(200).min(1000);
    let iterations = tokio::task::spawn_blocking(move || {
        let end = Instant::now() + Duration::from_millis(ms);
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut n: u64 = 0;
        while Instant::now() < end {
            for _ in 0..10_000 {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                n += 1;
            }
            black_box(x);
        }
        n
    })
    .await
    .unwrap_or(0);
    Json(serde_json::json!({ "burned_ms": ms, "iterations": iterations }))
}

async fn init_db(db: &PgPool) -> Result<(), sqlx::Error> {
    // Advisory lock so several instances booting at once don't race on create/seed.
    let mut tx = db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(424242)").execute(&mut *tx).await?;
    sqlx::raw_sql(SCHEMA).execute(&mut *tx).await?;
    tx.commit().await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
    let frontend_dir = env::var("FRONTEND_DIR").unwrap_or_else(|_| "frontend".into());

    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("failed to connect to database");
    init_db(&db).await.expect("failed to initialise schema");

    let state = AppState {
        db,
        info: Arc::new(InstanceInfo {
            instance_id: env::var("INSTANCE_ID").unwrap_or_else(|_| "local".into()),
            availability_zone: env::var("AVAILABILITY_ZONE").unwrap_or_else(|_| "local".into()),
        }),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/info", get(info))
        .route("/api/books", get(list_books))
        .route("/api/orders", get(list_orders).post(create_order))
        .route("/api/stress", get(stress))
        .fallback_service(ServeDir::new(frontend_dir))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    tracing::info!("listening on {port}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; })
        .await
        .unwrap();
}
