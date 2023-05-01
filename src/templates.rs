use askama::Template;

use crate::CacheType;

#[derive(Template, Debug, Clone, Eq, PartialEq)]
#[template(path = "location.jinja")]
pub struct Location {
    pub location: String,
    pub domain: Option<String>,
    pub alias: String,
    pub fallback: bool,
    pub basic_auth: Option<String>,
    pub cache_type: CacheType,
}

#[derive(Template, Debug, Clone, Eq, PartialEq)]
#[template(path = "server.jinja")]
pub struct Server {
    pub domain: Option<String>,
    pub locations: Vec<Location>,
}
