use std::io::Cursor;
use std::usize;
use axum::response::IntoResponse;
use axum::{
    body::Body,
    extract::Path,
    routing::{get, put},
    Router,
};
use foundationdb::RangeOption;
use futures::stream::StreamExt;
use byteorder::{BigEndian, ReadBytesExt};

async fn get_object<'a>() -> &'a str {
    "Hello, World! "
}

async fn download(Path(file_name): Path<String>) -> impl IntoResponse {
    let db = foundationdb::Database::default().unwrap();

    let trx = db.create_trx().unwrap();
    let begin = format!("{}/data/", &file_name);
    let end = format!("{}/datb", &file_name);
    let opt = RangeOption::from((
        begin.as_bytes(),
        end.as_bytes(),
    ));

    let mut x = trx.get_ranges_keyvalues(opt, false);

    let size = trx.get(format!("{}/size", &file_name).as_bytes(), false).await;
    let vec = size.unwrap().unwrap().to_vec();
    let i = Cursor::new(vec).read_uint::<BigEndian>(8).unwrap();
    println!("download file {} with size {}", &file_name, i);
    let mut vec = vec![];
    while let Some(message) = x.next().await {
        let value = message.unwrap();
        let data = value.value();
        vec.extend(data)
    }

    let body = Body::from(vec);
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
        println!("{}-{} : {}", &file_name, part, data.len());
        transaction.set(format!("{}/data/{}", &file_name, part).as_bytes(), data);
        size += data.len();
        part = part + 1;
    }
    transaction.set(
        format!("{}/size", &file_name).as_bytes(),
        &size.to_ne_bytes(),
    );

    let commit = transaction.commit().await;

    match commit {
        Ok(_) => {println!("commit success")},
        Err(e) => {eprintln!("commit failed, {}", e)}
    }

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
