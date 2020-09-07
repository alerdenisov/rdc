//! RDC Web service implementation

use crate::utils::stringify;
use crate::Result;
use futures::stream::StreamExt;
use hyper::{Body, Method, Request, Response, StatusCode, Uri};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

#[derive(Debug, Serialize, Deserialize)]
struct FileDefinition {
  pub url: String,
  pub filename: String,
}

/// RDC Web service function to pass into make_service_fn
pub async fn web_service(req: Request<Body>) -> Result<Response<Body>> {
  log::debug!("{:?}", req);
  match (req.method(), req.uri().path()) {
    (&Method::POST, "/zip") => {
      let buf = hyper::body::to_bytes(req.into_body()).await?;

      let definitions = serde_json::from_slice::<Vec<FileDefinition>>(&buf)?;
      log::debug!("{:?}", definitions);

      // Creates temporary file
      let file = tempfile::tempfile()?;
      let mut reader = file.try_clone()?;

      // Create archive writer
      let mut zip = zip::ZipWriter::new(file);
      let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

      // Begin of downloading files from definitions list
      let download = futures::stream::iter(definitions.into_iter().map(|def| async move {
        let client = hyper::Client::new();
        let uri = def.url.as_str().parse::<Uri>().unwrap();
        let res = client.get(uri).await.unwrap();
        let buf = hyper::body::to_bytes(res.into_body()).await.unwrap();

        (def.filename, buf.to_vec())
      }))
      .buffer_unordered(8)
      .map(move |item| {
        zip.start_file(item.0, options).unwrap();
        zip.write(item.1.as_slice()).unwrap();
        zip.finish().unwrap();
        let buf: &mut [u8] = &mut [0u8; 65536];
        reader.read(buf).unwrap();
        Result::Ok(buf.to_vec())
      });

      let body = hyper::Body::wrap_stream(download);

      return Response::builder()
        .status(StatusCode::OK)
        .body(body)
        .map_err(stringify);
    }

    (method, path) => log::error!("Unhandled request: {}, {}", method, path),
  }

  Response::builder()
    .status(StatusCode::OK)
    .body("Hello wolrd".into())
    .map_err(stringify)
}
