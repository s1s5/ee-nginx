use std::path::{Path, PathBuf};

use base64::{engine::general_purpose, Engine};
use openssl::hash::{Hasher, MessageDigest};
use url::Url;

pub fn get_domain(domain: Option<&str>) -> Option<String> {
    let domain = domain.unwrap();
    if domain == "*" {
        None
    } else {
        Some(domain.to_string())
    }
}

pub fn get_scheme_and_domain_from_uri(uri: &Url) -> Option<String> {
    if let Some(domain) = uri.domain() {
        if domain == "*" {
            return None;
        }
    }
    let mut uri = uri.clone();
    uri.set_path("");
    uri.set_query(None);

    Some(uri.to_string().trim_end_matches("/").to_string())
}

pub fn force_append_trailing_slash(path: &str) -> String {
    if path.ends_with("/") {
        path.to_string()
    } else {
        format!("{}/", path)
    }
}

pub fn get_basic_auth_file_path(target_dir: &Path, username: &str, password: &str) -> PathBuf {
    let digest = {
        let mut hasher = Hasher::new(MessageDigest::sha1()).unwrap();
        hasher.update(username.as_bytes()).unwrap();
        hasher.update(password.as_bytes()).unwrap();
        hasher.finish().unwrap()
    };
    let hashed = general_purpose::URL_SAFE_NO_PAD.encode(digest);

    target_dir.join(hashed)
}
