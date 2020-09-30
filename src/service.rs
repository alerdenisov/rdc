//! RDC Web service implementation

use crate::utils::stringify;
use crate::Result;
use futures::stream::StreamExt;
use hyper::body::Bytes;
use hyper::{Body, Method, Request, Response, StatusCode, Uri};
use hyper_tls::HttpsConnector;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct FileDefinition {
  pub url: String,
  pub filename: String,
}

enum ZippingTask {
  File(String),
  Chunk(Bytes),
  Close,
  Panic,
}

fn random_id() -> String {
  format!("{}", Uuid::new_v4().to_simple())
}

pub async fn web_service(req: Request<Body>) -> Result<Response<Body>> {
  log::debug!("{:?}", req);
  match (req.method(), req.uri().path()) {
    (&Method::GET, "/sample.zip") => {
      log::info!("Request sample.zip");
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

async fn download_and_response(files: Vec<FileDefinition>) -> Result<Response<Body>> {
  // Creates temporary file
  // let file = tempfile::tempfile()?;
  let path = format!(".tmp/{}.zip", random_id());

  // check if file exists
  std::fs::remove_file(&path).ok();

  let file = std::fs::File::create(&path)?;

  let mut file_reader = std::fs::File::open(&path)?;

  let mut zip = zip::ZipWriter::new(file);
  let options =
    zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

  zip.start_file("files.json", options)?;
  zip.write(serde_json::to_string_pretty(&files)?.as_bytes())?;

  let stream =
    futures::stream::iter(files.into_iter().map(move |def| async move {
      log::info!("Loading file: {}", def.url);

      let https = HttpsConnector::new();
      let client = hyper::Client::builder().build::<_, Body>(https);
      let uri = def.url.as_str().parse::<Uri>()?;
      let res = client.get(uri).await?;
      Result::Ok((def.filename, res.into_body()))
    }))
    .buffered(8)
    .map(|r| match r {
      Ok((filename, reader)) => futures::stream::once(async { ZippingTask::File(filename) }).chain(
        reader.map(move |chunk| match chunk {
          Ok(bytes) => ZippingTask::Chunk(bytes),
          Err(_) => ZippingTask::Panic,
        }),
      ),
      Err(e) => {
        panic!("{}", e);
      }
    })
    .flatten()
    .chain(futures::stream::once(async { ZippingTask::Close }))
    .map(move |task| {
      match task {
        ZippingTask::File(filename) => {
          zip.start_file(&filename, options)?;
        }
        ZippingTask::Chunk(bytes) => {
          zip.write(&bytes)?;
        }
        ZippingTask::Close => {
          zip.finish()?;
        }
        ZippingTask::Panic => panic!(),
      }

      Result::Ok(())
    })
    .map(move |_| {
      let mut buf = Vec::new();
      file_reader.read_to_end(&mut buf)?;
      Result::Ok(buf)
    })
    .chain(futures::stream::once(async move {
      std::fs::remove_file(&path)?;
      Result::Ok(Vec::default())
    }));

  let body = hyper::Body::wrap_stream(stream);

  Response::builder()
    .status(StatusCode::OK)
    .body(body)
    .map_err(stringify)
}
