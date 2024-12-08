use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use bytes::{BufMut, BytesMut};
use foundationdb::directory::DirectoryLayer;
use foundationdb::directory::Directory;
use foundationdb::RangeOption;
use futures::stream::StreamExt;

use crate::{AppState, DATA_PREFIX, MAX_SIZE};

pub async fn download(
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

pub async fn put_object(
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

            // probably can be merged with next if
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
