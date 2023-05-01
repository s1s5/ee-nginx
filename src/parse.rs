use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use url::Url;

use crate::{
    error::CustomError,
    utils::{exclude_path_and_query_param, get_basic_auth_file_path, get_domain},
    CacheType, Location, ParsedResult, Server,
};

fn parse_cache_type(query: &str) -> CacheType {
    if query.contains("versioned") {
        CacheType::Versioned
    } else if query.contains("must-revalidate") {
        CacheType::MustRevalidate
    } else {
        CacheType::None
    }
}

pub fn parse(target_dir: &Path, env_var: &str) -> Result<ParsedResult, CustomError> {
    let api = Url::parse("file://*").unwrap();
    let parser = Url::options().base_url(Some(&api));

    let configs: Vec<&str> = env_var.split(|c| c == '\n' || c == ';').collect();
    let mut basic_auth_map: HashSet<(String, String)> = HashSet::new();
    let mut server_map: HashMap<String, Server> = HashMap::new();
    for conf in configs {
        let s: Vec<&str> = conf.split(|c| c == '>').collect();
        if s.len() != 2 {
            return Err(CustomError::new("must include one '>'"));
        }

        let s0 = parser.parse(s[0]).map_err(|e| {
            CustomError::new(format!(
                "location param invalid '{}', Error = {:?}",
                s[0], e
            ))
        })?;
        let s1 = parser.parse(s[1]).map_err(|e| {
            CustomError::new(format!("proxy param invalid '{}', Error = {:?}", s[1], e))
        })?;
        s0.domain()
            .ok_or(CustomError::new(format!("no domain found")))?;
        s1.domain()
            .ok_or(CustomError::new(format!("no domain found")))?;

        let basic_auth = if s0.username() != "" {
            let key = (
                s0.username().to_string(),
                s0.password()
                    .ok_or(CustomError::new(format!("no password set {}", s[0])))?
                    .to_string(),
            );
            let file_path = get_basic_auth_file_path(target_dir, &key.0, &key.1);
            basic_auth_map.insert(key);
            Some(file_path)
        } else {
            None
        };
        let loc = Location {
            location: s0.path().to_string(),
            domain: exclude_path_and_query_param(&s1),
            alias: s1.path().to_string(),
            fallback: s1.query().unwrap_or("").contains("fallback"),
            basic_auth: basic_auth.map(|x| x.to_str().unwrap().to_string()),
            cache_type: parse_cache_type(s1.query().unwrap_or("")),
        };

        let domain = s0.domain().unwrap();
        match server_map.get_mut(domain) {
            Some(server_conf) => {
                server_conf.locations.push(loc);
            }
            None => {
                server_map.insert(
                    domain.to_string(),
                    Server {
                        domain: get_domain(Some(domain)),
                        locations: vec![loc],
                    },
                );
            }
        }
    }

    Ok(ParsedResult {
        target_dir: PathBuf::from(target_dir),
        basic_auth_map,
        server_map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let target_dir = PathBuf::from("/etc/nginx/conf.d");
        for (conf_str, expected) in [
            (
                "/>/var/www/html?must-revalidate",
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::new(),
                    server_map: HashMap::from_iter([(
                        "*".to_string(),
                        Server {
                            domain: None,
                            locations: vec![Location {
                                location: "/".to_string(),
                                domain: None,
                                alias: "/var/www/html".to_string(),
                                fallback: false,
                                basic_auth: None,
                                cache_type: CacheType::MustRevalidate,
                            }],
                        },
                    )]),
                },
            ),
            (
                "/static>/var/www/html/;/>http://app:8000/",
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::new(),
                    server_map: HashMap::from_iter([(
                        "*".to_string(),
                        Server {
                            domain: None,
                            locations: vec![
                                Location {
                                    location: "/static".to_string(),
                                    domain: None,
                                    alias: "/var/www/html/".to_string(),
                                    fallback: false,
                                    basic_auth: None,
                                    cache_type: CacheType::None,
                                },
                                Location {
                                    location: "/".to_string(),
                                    domain: Some("http://app:8000".to_string()),
                                    alias: "/".to_string(),
                                    fallback: false,
                                    basic_auth: None,
                                    cache_type: CacheType::None,
                                },
                            ],
                        },
                    )]),
                },
            ),
            (
                "http://hoge.localhost>/var/www/html/hoge/;http://foo.localhost>/var/www/html/foo/",
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::new(),
                    server_map: HashMap::from_iter([
                        (
                            "hoge.localhost".to_string(),
                            Server {
                                domain: Some("hoge.localhost".to_string()),
                                locations: vec![Location {
                                    location: "/".to_string(),
                                    domain: None,
                                    alias: "/var/www/html/hoge/".to_string(),
                                    fallback: false,
                                    basic_auth: None,
                                    cache_type: CacheType::None,
                                }],
                            },
                        ),
                        (
                            "foo.localhost".to_string(),
                            Server {
                                domain: Some("foo.localhost".to_string()),
                                locations: vec![Location {
                                    location: "/".to_string(),
                                    domain: None,
                                    alias: "/var/www/html/foo/".to_string(),
                                    fallback: false,
                                    basic_auth: None,
                                    cache_type: CacheType::None,
                                }],
                            },
                        ),
                    ]),
                },
            ),
            (
                "http://user:password@*/secret>/var/www/html/secret/;http://user:password@foo.localhost>/var/www/html/foo/",
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::from_iter([("user".to_string(),"password".to_string())]),
                    server_map: HashMap::from_iter([
                        (
                            "*".to_string(),
                            Server {
                                domain: None,
                                locations: vec![Location {
                                    location: "/secret".to_string(),
                                    domain: None,
                                    alias: "/var/www/html/secret/".to_string(),
                                    fallback: false,
                                    basic_auth: Some("/etc/nginx/conf.d/xzuimCxVt-rQ5AmKkvcivbOjs9g".to_string()),
                                    cache_type: CacheType::None,
                                }],
                            },
                        ),
                        (
                            "foo.localhost".to_string(),
                            Server {
                                domain: Some("foo.localhost".to_string()),
                                locations: vec![Location {
                                    location: "/".to_string(),
                                    domain: None,
                                    alias: "/var/www/html/foo/".to_string(),
                                    fallback: false,
                                    basic_auth: Some("/etc/nginx/conf.d/xzuimCxVt-rQ5AmKkvcivbOjs9g".to_string()),
                                    cache_type: CacheType::None,
                                }],
                            },
                        ),
                    ]),
                },
            ),
        ] {
            let parsed_result = parse(&target_dir, conf_str).expect("parse failed");
            assert_eq!(parsed_result, expected);
        }
    }
}
