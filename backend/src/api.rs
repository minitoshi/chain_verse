use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::database::{Database, StoredKeyword, StoredPoem};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}

#[derive(Serialize)]
struct TodayStatus {
    date: String,
    keywords_collected: usize,
    keywords_needed: usize,
    poem_ready: bool,
    keywords: Vec<StoredKeyword>,
    poem: Option<StoredPoem>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

pub fn create_router(db: Database) -> Router {
    let state = AppState { db: Arc::new(db) };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .route("/api/poems", get(get_all_poems))
        .route("/api/poems/today", get(get_today))
        .route("/api/poems/{date}", get(get_poem_by_date))
        .route("/api/keywords/today", get(get_today_keywords))
        .with_state(state)
        .layer(cors)
}

/// GET /health - Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "chain_verse",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// GET /api/poems - Get all poems
async fn get_all_poems(
    State(state): State<AppState>,
) -> Result<Json<Vec<StoredPoem>>, (StatusCode, Json<ErrorResponse>)> {
    match state.db.get_all_poems().await {
        Ok(poems) => Ok(Json(poems)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

/// GET /api/poems/today - Get today's status (poem or in-progress)
async fn get_today(
    State(state): State<AppState>,
) -> Result<Json<TodayStatus>, (StatusCode, Json<ErrorResponse>)> {
    let today = Database::today();

    let keywords = match state.db.get_keywords_for_date(&today).await {
        Ok(kw) => kw,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    };

    let poem = match state.db.get_poem_by_date(&today).await {
        Ok(p) => p,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            ))
        }
    };

    Ok(Json(TodayStatus {
        date: today,
        keywords_collected: keywords.len(),
        keywords_needed: 15, // Target number
        poem_ready: poem.is_some(),
        keywords,
        poem,
    }))
}

/// GET /api/poems/:date - Get a specific poem by date
async fn get_poem_by_date(
    State(state): State<AppState>,
    Path(date): Path<String>,
) -> Result<Json<StoredPoem>, (StatusCode, Json<ErrorResponse>)> {
    match state.db.get_poem_by_date(&date).await {
        Ok(Some(poem)) => Ok(Json(poem)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("No poem found for date: {}", date),
            }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

/// GET /api/keywords/today - Get today's keywords
async fn get_today_keywords(
    State(state): State<AppState>,
) -> Result<Json<Vec<StoredKeyword>>, (StatusCode, Json<ErrorResponse>)> {
    let today = Database::today();

    match state.db.get_keywords_for_date(&today).await {
        Ok(keywords) => Ok(Json(keywords)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

pub async fn serve(db: Database, port: u16) -> anyhow::Result<()> {
    let app = create_router(db);

    let addr = format!("0.0.0.0:{}", port);
    println!("🌐 API server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
