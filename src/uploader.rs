use std::{path::PathBuf, time::Duration};

use once_cell::sync::Lazy;
use reqwest::{
    header::{HeaderMap, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE},
    Body, Client, Url,
};
use serde_json::{json, Value};
use tokio::fs::{metadata, File};
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::types::{BotError, BotResult};

pub async fn upload_audio(
    token: &String,
    file_title: &String,
    file_path: &PathBuf,
) -> BotResult<()> {
    let file_size = metadata(file_path).await?.len();
    // Pocket Casts API returns a S3 url to push the audio file to
    let upload_url = request_upload(token, file_title, file_size).await?;
    send_file(upload_url, file_path, file_size).await?;
    Ok(())
}

async fn request_upload(token: &String, file_name: &String, file_size: u64) -> BotResult<Url> {
    let url = String::from("https://api.pocketcasts.com/files/upload/request");
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, format!("Bearer {}", token).parse().unwrap());
    let request_body = json!({
        "contentType": "audio/mp4",
        "hasCustomImage": false,
        "title": file_name,
        "size": file_size,
    });
    // Initializes client once, reuses everytime afterwards. Otherwise reinitializing on every request is slow.
    let client = Lazy::new(|| {
        Client::builder()
            .build()
            .expect("Client for requesting upload failed to init")
    });
    let response = client
        .post(url)
        .timeout(Duration::new(5, 0))
        .headers(headers)
        .json(&request_body)
        .send()
        .await?
        .text()
        .await?;
    let parsed_response: Value = serde_json::from_str(&response).expect("HTTP POST failed");
    let response_url = parsed_response["url"]
        .as_str()
        .expect("Unable to convert url to str");
    let url = Url::parse(response_url).expect("Unable to parse request URL");
    Ok(url)
}

// TODO: Properly deal with Pocketcast errors, such as an invalid auth token or account storage is full.
async fn send_file(url: Url, file_path: &PathBuf, file_size: u64) -> BotResult<()> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "audio/mp4".parse().unwrap());
    headers.insert(CONTENT_LENGTH, file_size.to_string().parse().unwrap());
    let file = File::open(file_path).await?;
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = Body::wrap_stream(stream);
    // Initializes client once, reuses everytime afterwards. Otherwise reinitializing on every request is slow.
    let client = Lazy::new(|| {
        Client::builder()
            .build()
            .expect("Client for sending file failed to init")
    });
    let response = client
        .put(url)
        .timeout(Duration::new(5, 0))
        .headers(headers)
        .body(body)
        .send()
        .await?;
    if response.status().is_success() {
        return Ok(());
    } else {
        return Err(BotError::new(crate::types::BotErrorKind::UploadError));
    }
}
