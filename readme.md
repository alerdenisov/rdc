 # Rust Developer Challenge

 ## Introduction
 RDC is a simple Rust application that provides HTTP endpoints and responds to requests for files. Everything was done in accordance with the assignment received from Finheaven at the recruitment stage.

 ## Assigment
 Code up a simple http microservice that loads a JSON data structure like the one found below and responds with a .zip file whose contents are each of the source `url` named as `filename` within the final .zip archive.

 Your service should expose a URL and respond with data as soon as possible rather than make the user wait for the entire ZIP to be created first.

 Please include instructions on how someone else can run/test your service in their own webserver or development environment.

 ## Run

 Easiest way to test solution is run `cargo run` and open http://localhost:8080/sample.zip in browser.

 To process custom json send it with `curl`:
 ```bash
 curl --request POST --data-binary "@assets/sample_files.json" http://localhost:8080/zip > ~/Downloads/sample_files.zip
 ```

 ## Implementation and decisions

 ### HTTP-server
 I choice [hyper](hyper) as base library to listening TCP port for a [`Request`'s](hyper::Request). Hyper is low-level library and allows to manipulate with [`Response`](hyper::Response) object to pipe streams to it

 ### Archiving
 I choice [zip](zip) as library for archiving files

 ### Logging
 Every internal processes are intrumented with [`log`](log) calls with widely used channels: [`debug`](log::debug), [`info`](log::info), [`warn`](log::warn) and [`error`](log::error).

 ### Future improvements
 - Looks like it is possible to write non-finished incoming buffer to ZipWritter. It will improve UX
 - Use something such as `clap` to make service configurable
 - Checking of file name duplicates and make them unique
 - Per file caching