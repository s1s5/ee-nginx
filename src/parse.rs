use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use log::debug;
use url::Url;

use crate::{
    error::CustomError,
    templates::{Config, Location, Server},
    utils::{
        force_append_trailing_slash, get_basic_auth_file_path, get_domain,
        get_scheme_and_domain_from_uri,
    },
    CacheType, ParsedResult,
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

pub fn parse<'a>(
    target_dir: &Path,
    env_var: &str,
    config: &'a Config,
) -> Result<ParsedResult<'a>, CustomError> {
    let api = Url::parse("file://*").unwrap();
    let parser = Url::options().base_url(Some(&api));

    let configs: Vec<&str> = env_var
        .split(|c| c == '\n')
        .map(|x| x.trim())
        .filter(|x| !x.starts_with("#"))
        .flat_map(|l| l.split(|c| c == ';'))
        .map(|x| x.trim())
        .filter(|x| x.len() > 0)
        .collect();
    let mut basic_auth_map: HashSet<(String, String)> = HashSet::new();
    let mut server_map: HashMap<String, Server> = HashMap::new();
    for conf in configs {
        let s: Vec<&str> = conf
            .split(|c| c == '>')
            .map(|x| x.trim())
            .filter(|x| x.len() > 0)
            .collect();
        if s.len() != 2 {
            return Err(CustomError::new(format!(
                "must include one '>'. '{}'",
                conf
            )));
        }

        debug!("loading config: {} > {}", s[0], s[1]);

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
            config,
            location: s0.path().to_string(),
            domain: get_scheme_and_domain_from_uri(&s1),
            alias: force_append_trailing_slash(s1.path()),
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
                        config,
                        domain: get_domain(Some(domain)),
                        port: s0.port(),
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
    use crate::templates::Location;

    use super::*;

    #[test]
    fn test_parse() {
        let target_dir = PathBuf::from("/etc/nginx/conf.d");
        let config = Config { docker_mode: false };
        for (conf_str, expected) in [
            (
                "/>/var/www/html/?must-revalidate",
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::new(),
                    server_map: HashMap::from_iter([(
                        "*".to_string(),
                        Server {
                            config: &config,
                            domain: None,
                            port: None,
                            locations: vec![Location {
                                config: &config,
                                location: "/".to_string(),
                                domain: None,
                                alias: "/var/www/html/".to_string(),
                                fallback: false,
                                basic_auth: None,
                                cache_type: CacheType::MustRevalidate,
                            }],
                        },
                    )]),
                },
            ),
            (
                r#"
                /hello/ > /var/www/html/foo/
                "#,
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::new(),
                    server_map: HashMap::from_iter([(
                        "*".to_string(),
                        Server {
                            config: &config,
                            domain: None,
                            port: None,
                            locations: vec![Location {
                                config: &config,
                                location: "/hello/".to_string(),
                                domain: None,
                                alias: "/var/www/html/foo/".to_string(),
                                fallback: false,
                                basic_auth: None,
                                cache_type: CacheType::None,
                            }],
                        },
                    )]),
                },
            ),
            (
                "/test/foo > http://app:8000",
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::new(),
                    server_map: HashMap::from_iter([(
                        "*".to_string(),
                        Server {
                            config: &config,
                            domain: None,
                            port: None,
                            locations: vec![
                                Location {
                                    config: &config,
                                    location: "/test/foo".to_string(),
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
                "/static>/var/www/html/;/>http://app:8000/",
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::new(),
                    server_map: HashMap::from_iter([(
                        "*".to_string(),
                        Server {
                            config: &config,
                            domain: None,
                            port: None,
                            locations: vec![
                                Location {
                                    config: &config,
                                    location: "/static".to_string(),
                                    domain: None,
                                    alias: "/var/www/html/".to_string(),
                                    fallback: false,
                                    basic_auth: None,
                                    cache_type: CacheType::None,
                                },
                                Location {
                                    config: &config,
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
                r#"
                # static files location
                /static > /var/www/html/
                # app reverse proxy
                /       > http://app:8000/
                "#,
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::new(),
                    server_map: HashMap::from_iter([(
                        "*".to_string(),
                        Server {
                            config: &config,
                            domain: None,
                            port: None,
                            locations: vec![
                                Location {
                                    config: &config,
                                    location: "/static".to_string(),
                                    domain: None,
                                    alias: "/var/www/html/".to_string(),
                                    fallback: false,
                                    basic_auth: None,
                                    cache_type: CacheType::None,
                                },
                                Location {
                                    config: &config,
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
                "http://hoge.localhost:3333>/var/www/html/hoge/;http://foo.localhost>/var/www/html/foo/",
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::new(),
                    server_map: HashMap::from_iter([
                        (
                            "hoge.localhost".to_string(),
                            Server {
                                config: &config,
                                domain: Some("hoge.localhost".to_string()),
                                port: Some(3333),
                                locations: vec![Location {
                                    config: &config,
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
                                config: &config,
                                domain: Some("foo.localhost".to_string()),
                                port: None,
                                locations: vec![Location {
                                    config: &config,
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
                "http://user:password@*:8888/secret/>/var/www/html/secret/;http://user:password@foo.localhost>/var/www/html/foo/",
                ParsedResult {
                    target_dir: target_dir.clone(),
                    basic_auth_map: HashSet::from_iter([("user".to_string(),"password".to_string())]),
                    server_map: HashMap::from_iter([
                        (
                            "*".to_string(),
                            Server {
                                config: &config,
                                domain: None,
                                port: Some(8888),
                                locations: vec![Location {
                                    config: &config,
                                    location: "/secret/".to_string(),
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
                                config: &config,
                                domain: Some("foo.localhost".to_string()),
                                port: None,
                                locations: vec![Location {
                                    config: &config,
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
            let parsed_result = parse(&target_dir, conf_str, &config).expect("parse failed");
            assert_eq!(parsed_result, expected);
        }
    }
}
