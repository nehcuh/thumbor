pub(crate) mod engine;
pub(crate) mod pb;

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    net::SocketAddr,
    num::NonZero,
    sync::{Arc, Mutex},
};

use anyhow::Result as AnyResult;
use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue},
    routing::get,
};
use bytes::Bytes;
use engine::Engine;
use lru::LruCache;
use percent_encoding::percent_decode_str;
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing::instrument;

#[derive(Deserialize)]
struct Params {
    spec: String,
    url: String,
}
type Cache = Arc<Mutex<LruCache<u64, Bytes>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cache: Cache = Arc::new(Mutex::new(LruCache::new(NonZero::new(1024).unwrap())));

    let app = Router::new()
        .route("/image/{spec}/{url}", get(generate))
        .layer(TraceLayer::new_for_http())
        .with_state(cache);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);

    let listener = TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

async fn generate(
    Path(params): Path<Params>,
    State(cache): State<Cache>,
) -> Result<(HeaderMap, Vec<u8>), StatusCode> {
    let spec: crate::pb::abi::ImageSpec = params
        .spec
        .as_str()
        .try_into()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let url = percent_decode_str(&params.url).decode_utf8_lossy();
    let data = retrieve_image(&url, cache)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut engine: crate::engine::image_engine::ImageEngine = data
        .try_into()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    engine.apply(&spec.specs);

    // let image = engine.generate(image::ImageFormat::Jpeg);
    let image = engine.generate(image::ImageFormat::Png);
    info!("Finished processing: image size {}", image.len());

    let mut headers = HeaderMap::new();
    // headers.insert("CONTENT-TYPE", HeaderValue::from_static("image/jpeg"));
    headers.insert("CONTENT-TYPE", HeaderValue::from_static("image/png"));

    Ok((headers, image))
}

#[instrument(level = "info", skip(cache))]
async fn retrieve_image(url: &str, cache: Cache) -> AnyResult<Bytes> {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let key = hasher.finish();

    {
        let mut guard = cache.lock().unwrap();
        if let Some(data) = guard.get(&key) {
            info!("Match cached {}", key);
            return Ok(data.to_owned());
        }
    }

    // If not in cache, fetch it
    info!("Retrieve url");
    let resp = reqwest::get(url).await?;
    let data = resp.bytes().await?;

    // Then update the cache
    let mut guard = cache.lock().unwrap();
    guard.put(key, data.clone());

    Ok(data)
}
