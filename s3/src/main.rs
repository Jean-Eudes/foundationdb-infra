use axum::{
    body::Bytes,
    extract::Path,
    routing::{get, put},
    Router,
};

async fn get_object<'a>() -> &'a str {
    "Hello, World!"
}

async fn put_object(Path(file_name): Path<String>, body: Bytes) -> String {
    println!("{}", file_name);
    // println!(body.);
    format!("{file_name}")
}

#[tokio::main]
async fn main() {
    // build our application with a single route
    let router = Router::new()
        .route("/", get(get_object))
        .route("/:file_name", put(put_object));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
