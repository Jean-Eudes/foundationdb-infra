use std::io::Read;

use axum::response::IntoResponse;
use axum::{
    body::Body,
    extract::Path,
    routing::{get, put},
    Router,
};
use futures::stream::StreamExt;

async fn get_object<'a>() -> &'a str {
    "Hello, World! "
}

async fn download(Path(file_name): Path<String>) -> impl IntoResponse {
    let db = foundationdb::Database::default().unwrap();

    let trx = db.create_trx().unwrap();
    let time = trx.get(&file_name.as_bytes(), false).await.unwrap();

    let bytes = time
        .unwrap()
        .bytes()
        .collect::<Result<Vec<u8>, _>>()
        .unwrap();
    let body = Body::from(bytes);
    body
}

async fn put_object(Path(file_name): Path<String>, body: Body) -> String {
    let db = foundationdb::Database::default().unwrap();

    let mut stream = body.into_data_stream();

    let transaction = db.create_trx().unwrap();
    let mut part = 1;
    let mut size: usize = 0;
    while let Some(message) = stream.next().await {
        let data = &message.unwrap()[..];
        transaction.set(
            format!("{}/data/{}", &file_name, part).as_bytes(),
            data,
        );
        size += data.len();
        part = part + 1;
    }
    transaction.set(
        format!("{}/size", &file_name).as_bytes(),
        &size.to_ne_bytes(),
    );

    let _ = transaction.commit().await;

    file_name
}

#[tokio::main]
async fn main() {
    let network = unsafe { foundationdb::boot() };

    // build our application with a single route
    let router = Router::new()
        .route("/", get(get_object))
        .route("/:file_name", put(put_object))
        .route("/:file_name", get(download));

    println!("start service on port 3000");
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();

    drop(network);
}
