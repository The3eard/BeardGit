//! Pure logic layer for the Requests panel: `.http` parser, variable
//! resolver, HTTP executor (over `reqwest`), and code generators for
//! "Copy as cURL/fetch/HTTPie/wget".

pub mod codegen;
pub mod env;
pub mod error;
pub mod executor;
pub mod history;
pub mod parser;
pub mod resolver;
pub mod seed;
pub mod types;

pub use error::RequestsError;
pub use types::*;
