mod chunk;
mod komga;
mod parser;
mod state;

use axum::{
    Router,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
    routing::{get, patch, post},
    Json,
};
use serde::{Deserialize, Serialize};
use state::AppState;
use std::sync::Arc;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState::new());

    // Ensure progress directory exists
    if let Some(parent) = state.progress_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let app = Router::new()
        .route("/api/libraries", get(api_libraries))
        .route("/api/series", get(api_series))
        .route("/api/books", get(api_books))
        .route("/api/books/{id}/chunks", get(api_book_chunks))
        .route("/api/books/{id}/thumbnail", get(api_book_thumbnail))
        .route("/api/books/{id}/progress", get(api_get_progress))
        .route("/api/books/{id}/progress", patch(api_save_progress))
        .route("/api/books/{id}/bookmarks", post(api_save_bookmarks))
        .route("/api/upload", post(api_upload))
        .fallback_service(ServeDir::new("frontend").append_index_html_on_directories(true))
        .with_state(state);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000u16);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("Failed to bind");

    println!("Zen Reader listening on http://0.0.0.0:{port}");

    axum::serve(listener, app).await.expect("Server failed");
}

// --- API handlers ---

async fn api_libraries(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match komga::get_libraries(&state).await {
        Ok(libs) => Json(libs).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

#[derive(Deserialize)]
struct SeriesQuery {
    #[serde(rename = "libraryId")]
    library_id: String,
}

async fn api_series(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SeriesQuery>,
) -> impl IntoResponse {
    match komga::get_series(&state, &q.library_id).await {
        Ok(series) => Json(series).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

#[derive(Deserialize)]
struct BooksQuery {
    #[serde(rename = "seriesId")]
    series_id: String,
}

async fn api_books(
    State(state): State<Arc<AppState>>,
    Query(q): Query<BooksQuery>,
) -> impl IntoResponse {
    match komga::get_books(&state, &q.series_id).await {
        Ok(books) => Json(books).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

async fn api_book_chunks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let epub_data = match komga::download_epub(&state, &id).await {
        Ok(data) => data,
        Err(e) => return (StatusCode::BAD_GATEWAY, e).into_response(),
    };

    match parser::parse_epub(&epub_data) {
        Ok(chunks) => Json(chunks).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn api_book_thumbnail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match komga::download_thumbnail(&state, &id).await {
        Ok((bytes, content_type)) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, content_type.parse().unwrap());
            headers.insert(header::CACHE_CONTROL, "max-age=86400".parse().unwrap());
            (headers, bytes).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, e).into_response(),
    }
}

async fn api_upload(mut multipart: Multipart) -> impl IntoResponse {
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            let data = match field.bytes().await {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    return (StatusCode::BAD_REQUEST, format!("Upload failed: {e}"))
                        .into_response()
                }
            };
            return match parser::parse_epub(&data) {
                Ok(chunks) => Json(chunks).into_response(),
                Err(e) => (StatusCode::UNPROCESSABLE_ENTITY, e).into_response(),
            };
        }
    }
    (StatusCode::BAD_REQUEST, "No file field in upload").into_response()
}

// --- Progress ---

#[derive(Serialize, Deserialize, Default)]
struct ProgressStore {
    #[serde(flatten)]
    books: std::collections::HashMap<String, ProgressEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ProgressEntry {
    chunk_index: usize,
    total_chunks: usize,
    last_read: String,
    #[serde(default)]
    bookmarks: Vec<Bookmark>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Bookmark {
    chunk_index: usize,
    color: String,
    #[serde(default)]
    label: String,
}

fn load_progress(path: &std::path::Path) -> ProgressStore {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_progress_store(path: &std::path::Path, store: &ProgressStore) -> Result<(), String> {
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

async fn api_get_progress(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let store = load_progress(&state.progress_path);
    match store.books.get(&id) {
        Some(entry) => Json(serde_json::json!({
            "chunk_index": entry.chunk_index,
            "total_chunks": entry.total_chunks,
            "bookmarks": entry.bookmarks,
        }))
        .into_response(),
        None => Json(serde_json::json!({
            "chunk_index": 0,
            "total_chunks": 0,
            "bookmarks": [],
        }))
        .into_response(),
    }
}

#[derive(Deserialize)]
struct ProgressUpdate {
    chunk_index: usize,
    total_chunks: usize,
}

async fn api_save_progress(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(update): Json<ProgressUpdate>,
) -> impl IntoResponse {
    let mut store = load_progress(&state.progress_path);
    let bookmarks = store
        .books
        .get(&id)
        .map(|e| e.bookmarks.clone())
        .unwrap_or_default();
    store.books.insert(
        id,
        ProgressEntry {
            chunk_index: update.chunk_index,
            total_chunks: update.total_chunks,
            last_read: chrono::Utc::now().to_rfc3339(),
            bookmarks,
        },
    );
    match save_progress_store(&state.progress_path, &store) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn api_save_bookmarks(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(bookmarks): Json<Vec<Bookmark>>,
) -> impl IntoResponse {
    let mut store = load_progress(&state.progress_path);
    if let Some(entry) = store.books.get_mut(&id) {
        entry.bookmarks = bookmarks;
    } else {
        store.books.insert(
            id,
            ProgressEntry {
                chunk_index: 0,
                total_chunks: 0,
                last_read: chrono::Utc::now().to_rfc3339(),
                bookmarks,
            },
        );
    }
    match save_progress_store(&state.progress_path, &store) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}
