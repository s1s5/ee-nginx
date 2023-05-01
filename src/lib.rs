use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use askama::Template;

mod error;
mod output;
mod parse;
mod utils;

#[derive(Debug, Clone, Eq, PartialEq)]
enum CacheType {
    None,
    MustRevalidate,
    Versioned,
}

#[derive(Template, Debug, Clone, Eq, PartialEq)]
#[template(path = "location.jinja")]
struct Location {
    location: String,
    domain: Option<String>,
    alias: String,
    fallback: bool,
    basic_auth: Option<String>,
    cache_type: CacheType,
}

#[derive(Template, Debug, Clone, Eq, PartialEq)]
#[template(path = "server.jinja")]
struct Server {
    domain: Option<String>,
    locations: Vec<Location>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedResult {
    target_dir: PathBuf,
    basic_auth_map: HashSet<(String, String)>,
    server_map: HashMap<String, Server>,
}

pub use output::output;
pub use parse::parse;
