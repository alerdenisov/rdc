//! RDC Web service implementation

use crate::utils::stringify;
use crate::Result;
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

/// RDC Web service function to pass into make_service_fn
pub async fn web_service(req: Request<Body>) -> Result<Response<Body>> {
  log::debug!("{:?}", req);
  match (req.method(), req.uri().path()) {
    (&Method::GET, "/sample.zip") => {
      let mut sample = std::fs::File::open("assets/small.json")?;
      let mut json = String::new();
      sample.read_to_string(&mut json)?;
      let definitions = serde_json::from_str(&json)?;
      let key = format!("{:x}", md5::compute(json.as_bytes()));
      return download_and_response(&key, definitions).await;
    }
    (&Method::POST, "/zip") => {
      let buf = hyper::body::to_bytes(req.into_body()).await?;
      let key = format!("{:x}", md5::compute(&buf));
      return download_and_response(&key, serde_json::from_slice::<Vec<FileDefinition>>(&buf)?)
        .await;
    }

    (method, path) => log::error!("Unhandled request: {}, {}", method, path),
  }

  Response::builder()
    .status(StatusCode::OK)
    .body("Hello wolrd".into())
    .map_err(stringify)
}

async fn download_and_response(key: &str, files: Vec<FileDefinition>) -> Result<Response<Body>> {
  // Creates temporary file
  // let file = tempfile::tempfile()?;
  let path = format!(".cache/{}.zip", key);
  let path = std::path::Path::new(&path);

  // Use cached archive if available
  if path.is_file() {
    log::info!("Use cached version");
    let file = tokio::fs::File::open(&path).await?;
    let reader = FramedRead::new(file, BytesCodec::new());

    let body = hyper::Body::wrap_stream(reader);

    return Response::builder()
      .status(StatusCode::OK)
      .body(body)
      .map_err(stringify);
  }
  let file = std::fs::File::create(&path)?;
  let mut file_reader = std::fs::File::open(&path)?;

  let mut zip = zip::ZipWriter::new(file);
  let options =
    zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

  zip.start_file("files.json", options)?;
  zip.write(serde_json::to_string_pretty(&files)?.as_bytes())?;

  let mut last: String = String::new();
  let stream = futures::stream::iter(files.into_iter().map(move |def| async move {
    log::info!("Loading file: {}", def.url);

    let https = HttpsConnector::new();
    let client = hyper::Client::builder().build::<_, Body>(https);
    let uri = def.url.as_str().parse::<Uri>()?;
    let res = client.get(uri).await?;
    Result::Ok((def.filename, res.into_body()))
  }))
  .buffered(8)
  .map(|r| match r {
    Ok((filename, reader)) => reader.map(move |chunk| (filename.clone(), Some(chunk), false)),
    Err(e) => {
      panic!("{}", e);
    }
  })
  .flatten()
  .chain(futures::stream::once(async { (String::new(), None, true) }))
  .map(move |(filename, chunk, end)| {
    if let Some(chunk) = chunk {
      let bytes = chunk?;
      if last != filename {
        zip.start_file(&filename, options)?;
        last = filename.clone();
      }

      let seek = zip.write(&bytes)?;
      Result::Ok(seek)
    } else {
      if end {
        zip.finish()?;
      }

      Result::Ok(0)
    }
  })
  .map(move |writed| {
    let seek = writed?;
    log::debug!("Seek file to {}", seek);
    let mut buf = Vec::new();
    file_reader.read_to_end(&mut buf)?;
    Result::Ok(buf)
  })
  .map(move |result| match result {
    Ok(ok) => Result::Ok(ok),
    Err(e) => {
      log::error!("{}", e);
      Result::Err(e)
    }
  });

  let body = hyper::Body::wrap_stream(stream);

  Response::builder()
    .status(StatusCode::OK)
    .body(body)
    .map_err(stringify)
}
