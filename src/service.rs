//! RDC Web service implementation

use crate::utils::stringify;
use crate::Result;
use futures::prelude::*;
use futures::stream::StreamExt;
use hyper::{Body, Method, Request, Response, StatusCode, Uri};
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Debug, Serialize, Deserialize)]
struct FileDefinition {
  pub url: String,
  pub filename: String,
}

async fn download_and_response(files: Vec<FileDefinition>) -> Result<Response<Body>> {
  // Creates temporary file
  std::fs::remove_file("../last.zip").ok();
  let file = std::fs::File::create("../last.zip")?;
  let mut zip = zip::ZipWriter::new(file.try_clone()?);
  let options =
    zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

  zip.start_file("files.json", options)?;
  zip.write(serde_json::to_string_pretty(&files)?.as_bytes())?;

  let mut reader = FramedRead::new(
    tokio::fs::File::open("../last.zip").await?,
    BytesCodec::new(),
  );

  // Begin of downloading files from definitions list
  let mut left = files.len();
  let stream = futures::stream::iter(files.into_iter().map(move |def| async move {
    let https = HttpsConnector::new();
    let client = hyper::Client::builder().build::<_, Body>(https);
    let uri = def.url.as_str().parse::<Uri>().unwrap();
    let res = client.get(uri).await.unwrap();
    let buf = hyper::body::to_bytes(res.into_body()).await.unwrap();

    (def.filename, buf.to_vec())
  }))
  .buffered(8)
  .map(move |(filename, data)| {
    log::info!("Loaded file {} is {} long", filename, data.len());
    zip.start_file(filename, options)?;
    let seek = zip.write(data.as_slice())?;

    left -= 1;

    if left == 0 {
      log::info!("Archive is done");
      zip.finish()?;
    // let mut buf = Vec::new();
    // reader.read_to_end(&mut buf)?;
    // log::debug!("{:?}", buf);
    // Result::Ok(buf)
    } else {
      // let mut buf = Vec::with_capacity(seek);
      // reader.read_exact(&mut buf)?;
      // log::debug!("{:?}", buf);
      // Result::Ok(buf)
    }

    Result::Ok(())
  });

  let body = hyper::Body::wrap_stream(reader);
  Response::builder()
    .status(StatusCode::OK)
    .body(body)
    .map_err(stringify)
}

/// RDC Web service function to pass into make_service_fn
pub async fn web_service(req: Request<Body>) -> Result<Response<Body>> {
  log::debug!("{:?}", req);
  match (req.method(), req.uri().path()) {
    (&Method::GET, "/sample.zip") => {
      let mut sample = std::fs::File::open("assets/small.json")?;
      let mut json = String::new();
      sample.read_to_string(&mut json)?;
      let definitions = serde_json::from_str(&json)?;
      return download_and_response(definitions).await;
    }
    (&Method::POST, "/zip") => {
      let buf = hyper::body::to_bytes(req.into_body()).await?;
      return download_and_response(serde_json::from_slice::<Vec<FileDefinition>>(&buf)?).await;
    }

    (method, path) => log::error!("Unhandled request: {}, {}", method, path),
  }

  Response::builder()
    .status(StatusCode::OK)
    .body("Hello wolrd".into())
    .map_err(stringify)
}
