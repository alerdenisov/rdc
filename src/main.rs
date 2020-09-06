#![deny(missing_docs)]
#![deny(missing_debug_implementations)]

//! # Rust Developer Challenge
//!
//! ## Introduction
//! RDC is a simple Rust application that provides HTTP endpoints and responds to requests for files. Everything was done in accordance with the assignment received from Finheaven at the recruitment stage.
//!
//! ## Assigment
//! Code up a simple http microservice that loads a JSON data structure like the one found below and responds with a .zip file whose contents are each of the source `url` named as `filename` within the final .zip archive.
//!
//! Your service should expose a URL and respond with data as soon as possible rather than make the user wait for the entire ZIP to be created first.
//!
//! Please include instructions on how someone else can run/test your service in their own webserver or development environment.
//!
//! ## Run and test
//!
//! <TODO>
//!
//! ## Implementation and decisions
//!
//! ### HTTP-server
//! I choice [hyper](hyper) as base library to listening TCP port for a [`Request`'s](hyper::Request). Hyper is low-level library and allows to manipulate with [`Response`](hyper::Response) object to pipe streams to it
//!
//! ### Archiving
//! <TODO>
//!
//! ### Logging
//! Every internal processes are intrumented with [`log`](log) calls with widely used channels: [`debug`](log::debug), [`info`](log::info), [`warn`](log::warn) and [`error`](log::error).

mod utils;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use utils::stringify;

type Result<T> = std::result::Result<T, failure::Error>;

#[tokio::main]
async fn main() -> Result<()> {
  // initialize logger
  env_logger::init();

  let addr = "127.0.0.1:8080".parse()?;

  let make_service =
    make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(response_examples)) });

  Server::bind(&addr)
    .serve(make_service)
    .await
    .map_err(stringify)
}

// TODO: use clap or something to make it configurable
static FILES: &str = "assets/sample_files.json";
static NOTFOUND: &[u8] = b"Not Found";

async fn response_examples(req: Request<Body>) -> Result<Response<Body>> {
  match (req.method(), req.uri().path()) {
    (&Method::GET, "/") | (&Method::GET, "/files.json") => simple_file_send(FILES).await,
    (&Method::GET, "/no_file.html") => {
      // Test what happens when file cannot be be found
      simple_file_send("this_file_should_not_exist.html").await
    }
    _ => Ok(not_found()),
  }
}

/// HTTP status code 404
fn not_found() -> Response<Body> {
  Response::builder()
    .status(StatusCode::NOT_FOUND)
    .body(NOTFOUND.into())
    .unwrap()
}

async fn simple_file_send(filename: &str) -> Result<Response<Body>> {
  // Serve a file by asynchronously reading it by chunks using tokio-util crate.

  if let Ok(file) = File::open(filename).await {
    let stream = FramedRead::new(file, BytesCodec::new());
    let body = Body::wrap_stream(stream);
    return Ok(Response::new(body));
  }

  Ok(not_found())
}
