use axum::{
    routing::{get, put},
    Router,
};

async fn get_object<'a>() -> &'a str {
    "Hello, World!"
}

async fn put_object<'a>() -> &'a str {
    "Hello, World put"
}

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new()
        .route("/", get(get_object))
        .route("/", put(put_object));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
