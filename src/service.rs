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

  // Begin of downloading files from definitions list
  let len = files.len();
  let mut index = 0;
  let stream = futures::stream::iter(files.into_iter().map(move |def| async move {
    log::info!("Loading file: {}", def.url);

    let https = HttpsConnector::new();
    let client = hyper::Client::builder().build::<_, Body>(https);
    let uri = def.url.as_str().parse::<Uri>().unwrap();
    let res = client.get(uri).await.unwrap();
    let buf = hyper::body::to_bytes(res.into_body()).await.unwrap();

    (def.filename, buf.to_vec())
  }))
  .buffer_unordered(1)
  .map(move |(filename, data)| {
    log::info!("Loaded file {} is {} long", filename, data.len());
    zip.start_file(filename, options)?;
    let position = zip.write(data.as_slice())? as u64;
    index += 1;

    if index == len {
      zip.finish()?;
    }

    Result::Ok((position, index == 1))
  })
  .map(move |result| match result {
    Ok((seek, _)) => {
      log::info!("Seek to file at: {}", seek);
      let mut buf = Vec::new();
      file_reader.read_to_end(&mut buf)?;
      Result::Ok(buf)
    }
    Err(e) => Result::Err(e),
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
