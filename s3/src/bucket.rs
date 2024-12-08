use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use foundationdb::directory::DirectoryLayer;
use foundationdb::directory::Directory;
use crate::AppState;

pub async fn get_bucket(State(state): State<AppState>, Path(bucket): Path<String>) -> StatusCode {
    let db = state.database;

    let directory = DirectoryLayer::default();
    let trx = db.create_trx().unwrap();

    let x = directory.exists(&trx, &[bucket]).await.unwrap();
    if x {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

pub async fn create_bucket(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
) -> impl IntoResponse {
    let db = state.database;

    let directory = DirectoryLayer::default();
    let trx = db.create_trx().unwrap();

    let new_bucket = directory.create(&trx, &[bucket], None, None).await;

    trx.commit().await.unwrap();
    match new_bucket {
        Ok(dir) => (StatusCode::CREATED, dir.get_path().join("/").to_string()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", e)),
    }
}