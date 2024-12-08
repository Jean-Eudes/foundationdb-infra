mod bucket;
mod object;

use crate::bucket::{create_bucket, get_bucket};
use crate::object::{download, put_object};
use axum::response::IntoResponse;
use axum::{
    routing::{get, put},
    Router,
};
use bytes::BufMut;
use foundationdb::Database;
use std::sync::Arc;
use std::usize;
use tokio::join;

const MAX_SIZE: usize = 90 * 1024;
const DATA_PREFIX: &'static str = "data";

#[derive(Clone)]
struct AppState {
    database: Arc<Database>,
}

async fn get_object<'a>() -> &'a str {
    "Hello, World!"
}

#[tokio::main]
async fn main() {
    let network = unsafe { foundationdb::boot() };

    let db = Database::default().unwrap();

    let state = AppState {
        database: Arc::new(db),
    };

    // build our application with a single route
    let router = Router::new()
        .route("/", get(get_object))
        .route("/:bucket/:file_name", put(put_object))
        .route("/:bucket/:file_name", get(download))
        .route("/:bucket", put(create_bucket))
        .route("/:bucket", get(get_bucket))
        .with_state(state.clone());

    let router_admin = Router::new()
        .route("/", get(get_object))
        .route("/:bucket", put(create_bucket))
        .route("/:bucket", get(get_bucket))
        .route("/:bucket/:file_name", put(put_object))
        .route("/:bucket/:file_name", get(download))
        .with_state(state);

    println!("start service on port 3000");
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let listener_admin = tokio::net::TcpListener::bind("0.0.0.0:4000").await.unwrap();
    let (a, b) = join!(
        axum::serve(listener, router),
        axum::serve(listener_admin, router_admin),
    );
    a.unwrap();
    b.unwrap();

    drop(network);
}
