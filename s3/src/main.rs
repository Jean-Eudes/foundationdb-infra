use std::io::Read;

use axum::{
    body::{Body, Bytes},
    extract::Path,
    routing::{get, put},
    Router,
};

use axum::response::IntoResponse;
async fn get_object<'a>() -> &'a str {
    "Hello, World!"
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
    // let bytes.as_slice();
    // let bytes = Bytes::from(pdf_buf);
    let body = Body::from(bytes);
    body
}

async fn put_object(Path(file_name): Path<String>, body: Bytes) -> String {
    let db = foundationdb::Database::default().unwrap();

    let value = &file_name;
    let tmp = &body;
    match db
        .run(|trx, _maybe_committed| async move {
            trx.set(&value.as_bytes(), &tmp[..]);
            Ok(())
        })
        .await
    {
        Ok(_) => println!("transaction committed"),
        Err(_) => eprintln!("cannot commit transaction"),
    };

    // println!(body.);
    "lol".to_string()
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
