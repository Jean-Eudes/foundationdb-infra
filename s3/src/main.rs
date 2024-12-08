use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    body::Body,
    extract::Path,
    routing::{get, put},
    Router,
};
use bytes::{BufMut, BytesMut};
use foundationdb::directory::{Directory, DirectoryLayer};
use foundationdb::{Database, RangeOption};
use futures::stream::StreamExt;
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

async fn get_bucket(State(state): State<AppState>, Path(bucket): Path<String>) -> StatusCode {
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

async fn create_bucket(
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

async fn download(
    State(state): State<AppState>,
    Path((bucket, file_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = state.database;

    let transaction = db.create_trx().unwrap();

    let directory = DirectoryLayer::default();
    let current_bucket = directory.open(&transaction, &[bucket], None).await;

    if let Ok(bucket) = current_bucket {
        let file_name_key = bucket.subspace(&(&file_name, DATA_PREFIX)).unwrap();

        let opt = RangeOption::from(file_name_key.range());

        let mut x = transaction.get_ranges_keyvalues(opt, false);

        let mut vec = vec![];
        while let Some(message) = x.next().await {
            let value = message.unwrap();
            let data = value.value();
            vec.put(data)
        }

        let body = Body::from(vec);
        Ok(body)
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "No bucket found".to_string(),
        ))
    }
}

async fn put_object(
    State(state): State<AppState>,
    Path((bucket, file_name)): Path<(String, String)>,
    body: Body,
) -> (StatusCode, String) {
    let db = state.database;

    println!("upload file {} {}", &bucket, &file_name);

    let mut stream = body.into_data_stream();
    let mut buffer = BytesMut::with_capacity(MAX_SIZE);
    let mut part = 1;
    let mut size: usize = 0;

    let transaction = db.create_trx().unwrap();
    let directory = DirectoryLayer::default();
    let current_bucket = directory.open(&transaction, &[bucket], None).await;

    if let Ok(bucket) = current_bucket {
        let data_key = bucket.subspace(&(&file_name, DATA_PREFIX)).unwrap();
        while let Some(message) = stream.next().await {
            let mut data = &message.unwrap()[..];
            size += data.len();
            if buffer.len() + data.len() < MAX_SIZE {
                buffer.put_slice(&data);
                continue;
            }

            if buffer.len() + data.len() == MAX_SIZE {
                buffer.put_slice(&data);
                transaction.set(&data_key.pack(&part), &buffer[..]);
                part = part + 1;
                buffer.clear();
                continue;
            }

            while buffer.len() + data.len() >= MAX_SIZE {
                let remaining_capacity = MAX_SIZE - buffer.len();
                buffer.put_slice(&data[0..remaining_capacity]);
                transaction.set(&data_key.pack(&part), &buffer[..]);
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
            transaction.set(&data_key.pack(&part), &buffer[..]);
        }

        println!("start commit");
        let commit = transaction.commit().await;
        println!("commit done");

        match commit {
            Ok(_) => {
                println!("commit success");
                (StatusCode::CREATED, file_name)
            }
            Err(e) => {
                eprintln!("commit failed, {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, file_name)
            }
        }
    } else {
        eprintln!("commit failed, {:?}", current_bucket);
        (StatusCode::BAD_REQUEST, "Bucket not found".to_string())
    }
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
