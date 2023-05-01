use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

mod error;
mod output;
mod parse;
mod templates;
mod utils;
pub use output::output;
pub use parse::parse;
use templates::Server;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CacheType {
    None,
    MustRevalidate,
    Versioned,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedResult {
    target_dir: PathBuf,
    basic_auth_map: HashSet<(String, String)>,
    server_map: HashMap<String, Server>,
}
