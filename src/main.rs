use askama::Template;
use base64::engine::general_purpose;
use base64::Engine;
use openssl::hash::{Hasher, MessageDigest};
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    path::{Path, PathBuf},
};
use url::Url;
// />/var/www/html
// />/var/www/html?versioned
// />/var/www/html?mustvalidate
// />app:8000
// /xxx>app:8000
// /xxx>app:8000/yyy
// /xxx>app:8000/yyy/
// foo.com>/data
// user:password@foo.com/hoge>app:800/yyyy
// /static>/var/www/static;/>app:8000
// />spa://app:3000

#[derive(Debug)]
pub struct CustomError {
    message: String,
}

impl CustomError {
    pub fn new(message: impl Into<String>) -> Self {
        CustomError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CustomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}", self.message)
    }
}

impl std::error::Error for CustomError {}

#[derive(Template, Debug, Clone, Eq, PartialEq)]
#[template(path = "location.jinja")]
struct Location {
    location: String,
    domain: Option<String>,
    alias: String,
    fallback: bool,
    basic_auth: Option<String>,
}

#[derive(Template, Debug, Clone, Eq, PartialEq)]
#[template(path = "server.jinja")]
struct Server {
    domain: Option<String>,
    locations: Vec<Location>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ParsedResult {
    target_dir: PathBuf,
    basic_auth_map: HashSet<(String, String)>,
    server_map: HashMap<String, Server>,
}

fn create_htpasswd(user: &str, password: &str) -> String {
    let mut hasher = Hasher::new(MessageDigest::sha1()).unwrap();
    hasher.update(password.as_bytes()).unwrap();
    let digest = hasher.finish().unwrap();

    let hashed = general_purpose::STANDARD.encode(digest);

    format!("{}:{{SHA}}{}", user, hashed)
}

fn get_domain(domain: Option<&str>) -> Option<String> {
    let domain = domain.unwrap();
    if domain == "*" {
        None
    } else {
        Some(domain.to_string())
    }
}

fn exclude_path_and_query_param(uri: &Url) -> Option<String> {
    let domain = uri.domain().unwrap();
    if domain == "*" {
        None
    } else {
        let mut uri = uri.clone();
        uri.set_path("");
        uri.set_query(None);

        Some(uri.to_string().trim_end_matches("/").to_string())
    }
}

fn get_basic_auth_file_path(target_dir: &Path, username: &str, password: &str) -> PathBuf {
    let digest = {
        let mut hasher = Hasher::new(MessageDigest::sha1()).unwrap();
        hasher.update(username.as_bytes()).unwrap();
        hasher.update(password.as_bytes()).unwrap();
        hasher.finish().unwrap()
    };
    let hashed = general_purpose::URL_SAFE_NO_PAD.encode(digest);

    target_dir.join(hashed)
}

fn write_to_file(dst_path: &Path, content: &str) -> Result<(), CustomError> {
    let mut file = std::fs::File::create(&dst_path)
        .map_err(|e| CustomError::new(format!("failed to crete file. {:?}, {:?}", dst_path, e)))?;
    file.write_all(content.as_bytes()).map_err(|e| {
        CustomError::new(format!(
            "write_all failed for path {:?}, Error:{:?}",
            dst_path, e
        ))
    })?;
    Ok(())
}

fn parse(target_dir: &Path, env_var: &str) -> Result<ParsedResult, CustomError> {
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

fn output(parsed_result: &ParsedResult) -> Result<(), CustomError> {
    for (user, password) in parsed_result.basic_auth_map.iter() {
        write_to_file(
            &get_basic_auth_file_path(&parsed_result.target_dir, user, password),
            &create_htpasswd(user, password),
        )?;
    }

    for (key, value) in parsed_result.server_map.iter() {
        let filename = if key == "*" {
            "default.conf".to_string()
        } else {
            format!("{}.conf", key.replace(".", "_"))
        };

        write_to_file(
            &parsed_result.target_dir.join(filename),
            &value.render().map_err(|e| {
                CustomError::new(format!("render failed {:?}, Error:{:?}", value, e))
            })?,
        )?;
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 2 {
        eprintln!("Usage: {} <output dir>", args[0]);
        std::process::exit(1);
    }

    let target_dir = PathBuf::from(if args.len() == 2 {
        &args[1]
    } else {
        "/etc/nginx/conf.d"
    });

    let parsed_result =
        parse(&target_dir, &std::env::var("NGINX_CONF").unwrap()).expect("parse failed");

    output(&parsed_result).expect("output failed");

    // use url::{ParseError, Url};
    // let options = Url::options();
    // let api = Url::parse("all://*").unwrap();
    // let base_url = options.base_url(Some(&api));

    // let test = base_url.parse("/var/www?fallback").unwrap();
    // println!("{:?}", test.query().unwrap().contains("fallback"));
    // println!("{:?}", api.domain().unwrap() != "*");

    // // println!("{:?}", base_url.parse("/"));
    // // println!("{:?}", base_url.parse("/xxx"));
    // // println!("{:?}", base_url.parse("foo.com/"));
    // // println!("{:?}", base_url.parse("user:pass@foo.com/"));
    // // println!("{:?}", base_url.parse("spa://foo.com/xxx"));
    // // println!("{:?}", base_url.parse("/var/www/html?versioned"));

    // println!(
    //     "{}",
    //     Location {
    //         location: "/static".to_string(),
    //         domain: None,
    //         path: "/mnt/hello".to_string(),
    //         fallback: true,
    //         basic_auth: None,
    //     }
    //     .render()
    //     .unwrap()
    // );

    // println!(
    //     "{}",
    //     Server {
    //         domain: None,
    //         locations: vec![Location {
    //             location: "/static".to_string(),
    //             domain: None,
    //             path: "/mnt/hello".to_string(),
    //             fallback: true,
    //             basic_auth: None,
    //         }],
    //     }
    //     .render()
    //     .unwrap()
    // )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let target_dir = PathBuf::from("/etc/nginx/conf.d");
        for (conf_str, expected) in [
            (
                "/>/var/www/html?mustvalidate",
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
                                },
                                Location {
                                    location: "/".to_string(),
                                    domain: Some("http://app:8000".to_string()),
                                    alias: "/".to_string(),
                                    fallback: false,
                                    basic_auth: None,
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

    // />/var/www/html
    // />/var/www/html?versioned
    // />/var/www/html?mustvalidate
    // />app:8000
    // /xxx>app:8000
    // /xxx>app:8000/yyy
    // /xxx>app:8000/yyy/
    // foo.com>/data
    // user:password@foo.com/hoge>app:800/yyyy
    // /static>/var/www/static;/>app:8000
    // />spa://app:3000
}
