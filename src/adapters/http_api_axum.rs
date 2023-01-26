use crate::app;
use axum::{
    body::{self, Full},
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    response::Response,
    routing::{delete, get, post},
    Json, Router,
};
use hyper::header;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(RustEmbed)]
#[folder = "assets/"]
struct Asset;

struct ServiceDependencies {
    clock: Arc<dyn app::Clock>,
    event_store: Arc<dyn app::EventStore>,
    read_model: Arc<dyn app::ReadModel>,
}

pub fn create_router(
    event_store: Arc<dyn app::EventStore>,
    read_model: Arc<dyn app::ReadModel>,
    clock: Arc<dyn app::Clock>,
) -> Router {
    let deps = Arc::new(ServiceDependencies {
        event_store: event_store.clone(),
        read_model: read_model.clone(),
        clock: clock.clone(),
    });

    Router::new()
        .route("/", get(root))
        .route("/api/bookmarks", get(read_bookmarks))
        .route("/api/bookmarks", post(create_bookmark))
        .route("/api/bookmarks/:id", get(read_bookmark))
        .route("/api/bookmarks/:id", delete(delete_bookmark))
        .with_state(deps)
}

async fn root(State(_state): State<Arc<ServiceDependencies>>) -> impl IntoResponse {
    let asset = Asset::get("index.html").unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html")
        .body(body::boxed(Full::from(asset.data)))
        .unwrap()
}

#[derive(Serialize)]
struct ReadBookmarksResponse {
    bookmarks: Vec<ReadBookmarksResponseBookmarkEntry>,
}
#[derive(Serialize)]
struct ReadBookmarksResponseBookmarkEntry {
    id: String,
    url: String,
    title: String,
}

async fn read_bookmarks(State(state): State<Arc<ServiceDependencies>>) -> impl IntoResponse {
    let bookmarks = app::query::read_bookmarks(state.read_model.clone()).unwrap();
    (
        StatusCode::OK,
        Json(ReadBookmarksResponse {
            bookmarks: bookmarks
                .iter()
                .map(|b| ReadBookmarksResponseBookmarkEntry {
                    id: b.id.clone(),
                    url: b.url.clone(),
                    title: b.title.clone(),
                })
                .collect(),
        }),
    )
}

#[derive(Deserialize)]
struct CreateBookmarkRequestPayload {
    url: String,
    title: String,
}

#[derive(Serialize)]
struct CreateBoomarkResponsePayload {
    id: String,
}

async fn create_bookmark(
    State(state): State<Arc<ServiceDependencies>>,
    Json(payload): Json<CreateBookmarkRequestPayload>,
) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();

    app::command::create_bookmark(
        app::Bookmark {
            id: id.clone(),
            url: payload.url,
            title: payload.title,
        },
        state.event_store.clone(),
        state.read_model.clone(),
        state.clock.clone(),
    )
    .unwrap();

    (
        StatusCode::CREATED,
        Json(CreateBoomarkResponsePayload { id }),
    )
}

async fn delete_bookmark(
    State(state): State<Arc<ServiceDependencies>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    app::command::delete_bookmark(
        id,
        state.event_store.clone(),
        state.read_model.clone(),
        state.clock.clone(),
    )
    .unwrap();

    (StatusCode::OK, ())
}

#[derive(Serialize)]
struct ReadBookmarkResponsePayload {
    id: String,
    url: String,
    title: String,
}

async fn read_bookmark(
    State(state): State<Arc<ServiceDependencies>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match app::query::read_bookmark(app::BookmarkQuery { id }, state.read_model.clone()) {
        Some(bookmark) => (
            StatusCode::OK,
            Json(ReadBookmarkResponsePayload {
                id: bookmark.id,
                url: bookmark.url,
                title: bookmark.title,
            }),
        )
            .into_response(),
        _ => (StatusCode::NOT_FOUND, ()).into_response(),
    }
}
