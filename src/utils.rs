//! # RDC Utils collection
//!

/// Retuns failure error from anything with [`Display`](std::fmt::Display) trait implemented
pub fn stringify<T: std::fmt::Display>(from: T) -> failure::Error {
  failure::format_err!("Error: {}", from)
}
