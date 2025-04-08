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
    Extension, Router,
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
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing::instrument;

#[derive(Deserialize)]
struct Params {
    specs: String,
    url: String,
}
type Cache = Arc<Mutex<LruCache<u64, Bytes>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Initialize the client state with a cache capacity of 100 entries.
    let client: Cache = Arc::new(Mutex::new(LruCache::new(NonZero::new(1024).unwrap())));

    // Axum 0.8+ requires correct route pattern format
    let app = Router::new()
        .route("/image/:specs/:url", get(generate))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .with_state(client); // Use with_state instead of Extension

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);

    let listener = TcpListener::bind(&addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn generate(
    Path(Params { specs, url }): Path<Params>,
    State(cache): State<Cache>,
) -> Result<(HeaderMap, Vec<u8>), StatusCode> {
    let specs: crate::pb::abi::ImageSpec = specs
        .as_str()
        .try_into()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let url = percent_decode_str(&url).decode_utf8_lossy();
    let data = retrieve_image(&url, cache)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut engine: crate::engine::image_engine::Photon = data
        .try_into()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    engine.apply(&specs.specs);

    let image = engine.generate(image::ImageFormat::Jpeg);
    info!("Finished processing: image size {}", image.len());

    let mut headers = HeaderMap::new();
    headers.insert("CONTENT-TYPE", HeaderValue::from_static("image/jpeg"));

    Ok((headers, image))
}

#[instrument(level = "info", skip(cache))]
async fn retrieve_image(url: &str, cache: Cache) -> AnyResult<Bytes> {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let key = hasher.finish();

    let mut g = cache.lock().unwrap();
    let data = match g.get(&key) {
        Some(v) => {
            info!("Match cached {}", key);
            v.to_owned()
        }
        None => {
            info!("Retrieve url");
            let resp = reqwest::get(url).await?;
            let data = resp.bytes().await?;
            g.put(key, data.clone());
            data
        }
    };
    Ok(data)
}
