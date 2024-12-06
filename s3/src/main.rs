use axum::response::IntoResponse;
use axum::{
    body::Body,
    extract::Path,
    routing::{get, put},
    Router,
};
use byteorder::{BigEndian, ReadBytesExt};
use bytes::{Buf, BufMut, BytesMut};
use foundationdb::RangeOption;
use futures::stream::StreamExt;
use std::io::Cursor;
use std::usize;

const MAX_SIZE: usize = 90 * 1024;

async fn get_object<'a>() -> &'a str {
    "Hello, World! "
}

async fn download(Path(file_name): Path<String>) -> impl IntoResponse {
    let db = foundationdb::Database::default().unwrap();

    let trx = db.create_trx().unwrap();
    let begin = format!("{}/data/", &file_name);
    let end = format!("{}/datb", &file_name);
    let opt = RangeOption::from((begin.as_bytes(), end.as_bytes()));

    let mut x = trx.get_ranges_keyvalues(opt, false);

    let size = trx
        .get(format!("{}/size", &file_name).as_bytes(), false)
        .await;
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
    let mut buffer = BytesMut::with_capacity(MAX_SIZE);

    let transaction = db.create_trx().unwrap();
    let mut part = 1;
    let mut size: usize = 0;
    while let Some(message) = stream.next().await {
        println!("download file {}", &file_name);
        let mut data = &message.unwrap()[..];
        size += data.len();
        println!("{}-{} : {}", &file_name, part, data.len());
        println!("remaining : {}", buffer.remaining());
        println!("capacity : {}", buffer.capacity());
        println!("len : {}", buffer.len());
        if buffer.len() + data.len() < MAX_SIZE {
            println!("put data in buffer");
            buffer.put_slice(&data);
            continue;
        }

        if buffer.len() + data.len() == MAX_SIZE {
            println!("put data in buffer and set");
            buffer.put_slice(&data);
            transaction.set(format!("{}/data/{}", &file_name, part).as_bytes(), &buffer[..]);
            part = part + 1;
            buffer.clear();
            continue;
        }

        while buffer.len() + data.len() >= MAX_SIZE {
            println!("put data in buffer, loop and set");

            let remaining_capacity = MAX_SIZE - buffer.len();
            buffer.put_slice(&data[0..remaining_capacity]);
            transaction.set(format!("{}/data/{}", &file_name, part).as_bytes(), &buffer[..]);
            buffer.clear();
            part = part + 1;
            data = &data[remaining_capacity..];
        }

        let remaining = &data[0..data.len()];
        buffer.put_slice(remaining);

    }
    transaction.set(
        format!("{}/size", &file_name).as_bytes(),
        &size.to_ne_bytes(),
    );

    if buffer.len() != 0 {
        println!("set last data {}", buffer[..].len());
        transaction.set(format!("{}/data/{}", &file_name, part).as_bytes(), &buffer[..]);
    }

    println!("start commit");
    let commit = transaction.commit().await;
    println!("commit done");

    match commit {
        Ok(_) => {
            println!("commit success")
        }
        Err(e) => {
            eprintln!("commit failed, {}", e)
        }
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
