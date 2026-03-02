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
pub use templates::Config;
use templates::Server;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CacheType {
    None,
    MustRevalidate,
    Versioned,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedResult<'a> {
    pub target_dir: PathBuf,
    pub basic_auth_map: HashSet<(String, String)>,
    pub server_map: HashMap<String, Server<'a>>,
}
