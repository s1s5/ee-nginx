use askama::Template;

use crate::CacheType;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Config {
    pub docker_mode: bool,
}

#[derive(Template, Debug, Clone, Eq, PartialEq)]
#[template(path = "location.jinja")]
pub struct Location<'a> {
    pub config: &'a Config,
    pub location: String,
    pub domain: Option<String>,
    pub alias: String,
    pub fallback: bool,
    pub basic_auth: Option<String>,
    pub cache_type: CacheType,
    pub nameserver: String,
    pub show_index: bool,
    pub is_file: bool,
}

#[derive(Template, Debug, Clone, Eq, PartialEq)]
#[template(path = "server.jinja")]
pub struct Server<'a> {
    pub config: &'a Config,
    pub domain: Option<String>,
    pub port: Option<u16>,
    pub locations: Vec<Location<'a>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_0() {
        let config = Config { docker_mode: false };
        assert_eq!(
            Location {
                config: &config,
                location: "/".to_string(),
                domain: None,
                alias: "/var/www/html/".to_string(),
                fallback: false,
                basic_auth: None,
                cache_type: CacheType::None,
                nameserver: "".to_string(),
                show_index: false,
                is_file: false,
            }
            .render()
            .expect("failed to render location"),
            r#"  location / {
    alias /var/www/html/;
    index index.html index.htm;
    add_header Cache-Control "no-store";
  }"#
        );
    }

    #[test]
    fn test_location_1() {
        let config = Config { docker_mode: false };
        assert_eq!(
            Location {
                config: &config,
                location: "/".to_string(),
                domain: None,
                alias: "/var/www/html/".to_string(),
                fallback: false,
                basic_auth: None,
                cache_type: CacheType::MustRevalidate,
                nameserver: "".to_string(),
                show_index: false,
                is_file: false,
            }
            .render()
            .expect("failed to render location"),
            r#"  location / {
    alias /var/www/html/;
    index index.html index.htm;
    add_header Cache-Control "no-cache";
  }"#
        );
    }

    #[test]
    fn test_location_2() {
        let config = Config { docker_mode: false };
        assert_eq!(
            Location {
                config: &config,
                location: "/".to_string(),
                domain: Some("http://app:8000".to_string()),
                alias: "/".to_string(),
                fallback: true,
                basic_auth: None,
                cache_type: CacheType::None,
                nameserver: "".to_string(),
                show_index: false,
                is_file: false,
            }
            .render()
            .expect("failed to render location"),
            r#"  location / {
    proxy_pass http://app:8000/;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_redirect off;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    try_files $uri $uri/ / =404;
  }"#
        );
    }

    #[test]
    fn test_location_3() {
        let config = Config { docker_mode: false };
        assert_eq!(
            Location {
                config: &config,
                location: "/".to_string(),
                domain: None,
                alias: "/var/www/html/".to_string(),
                fallback: false,
                basic_auth: Some("/etc/nginx/conf.d/htpasswd".to_string()),
                cache_type: CacheType::None,
                nameserver: "".to_string(),
                show_index: false,
                is_file: false,
            }
            .render()
            .expect("failed to render location"),
            r#"  location / {
    alias /var/www/html/;
    index index.html index.htm;
    add_header Cache-Control "no-store";
    auth_basic "Authorization required";
    auth_basic_user_file /etc/nginx/conf.d/htpasswd;
  }"#
        );
    }

    #[test]
    fn test_location_4() {
        let config = Config { docker_mode: false };
        assert_eq!(
            Location {
                config: &config,
                location: "/".to_string(),
                domain: None,
                alias: "/var/www/html/".to_string(),
                fallback: false,
                basic_auth: None,
                cache_type: CacheType::None,
                nameserver: "".to_string(),
                show_index: true,
                is_file: false,
            }
            .render()
            .expect("failed to render location"),
            r#"  location / {
    alias /var/www/html/;
    index index.html index.htm;
    add_header Cache-Control "no-store";
    autoindex on;
    autoindex_exact_size off;
    autoindex_localtime on;
  }"#
        );
    }

    #[test]
    fn test_location_file_0() {
        let config = Config { docker_mode: true };
        assert_eq!(
            Location {
                config: &config,
                location: "/config.json".to_string(),
                domain: None,
                alias: "/var/www/html/config.dev.json".to_string(),
                fallback: false,
                basic_auth: None,
                cache_type: CacheType::None,
                nameserver: "".to_string(),
                show_index: false,
                is_file: true,
            }
            .render()
            .expect("failed to render location"),
            r#"  location /config.json {
    alias /var/www/html/config.dev.json;
    index index.html index.htm;
    add_header Cache-Control "no-store";
  }"#
        );
    }

    #[test]
    fn test_location_file_2() {
        let config = Config { docker_mode: true };
        assert_eq!(
            Location {
                config: &config,
                location: "/config.json".to_string(),
                domain: Some("http://app:8000".to_string()),
                alias: "/config.stg.json".to_string(),
                fallback: true,
                basic_auth: None,
                cache_type: CacheType::None,
                nameserver: "127.0.0.11".to_string(),
                show_index: false,
                is_file: true,
            }
            .render()
            .expect("failed to render location"),
            r#"  location /config.json {
    resolver 127.0.0.11 valid=2s ipv6=off;
    set $target "http://app:8000";
    rewrite ^/config.json(.*)$ /config.stg.json$1 break;
    proxy_pass $target$uri$is_args$args;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_redirect off;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    try_files $uri $uri/ / =404;
  }"#
        );
    }


    #[test]
    fn test_server_0() {
        let config = Config { docker_mode: false };
        assert_eq!(
            Server {
                config: &config,
                domain: None,
                port: Some(99),
                locations: vec![]
            }
            .render()
            .expect("failed to render location"),
            r#"server {
  listen 99;
  client_max_body_size 1000M;
}"#
        );
    }

    #[test]
    fn test_server_1() {
        let config = Config { docker_mode: false };
        assert_eq!(
            Server {
                config: &config,
                domain: Some("foo.localhost".to_string()),
                port: None,
                locations: vec![]
            }
            .render()
            .expect("failed to render location"),
            r#"server {
  listen 80;
  client_max_body_size 1000M;
  server_name foo.localhost;
}"#
        );
    }
}
