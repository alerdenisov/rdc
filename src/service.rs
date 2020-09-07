//! RDC Web service implementation

use crate::utils::stringify;
use crate::Result;
use hyper::{Body, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};

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

    }

    (method, path) => log::error!("Unhandled request: {}, {}", method, path),
  }

  Response::builder()
    .status(StatusCode::OK)
    .body("Hello wolrd".into())
    .map_err(stringify)
}
