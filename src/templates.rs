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
    pub port: Option<u16>,
    pub locations: Vec<Location>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_0() {
        assert_eq!(
            Location {
                location: "/".to_string(),
                domain: None,
                alias: "/var/www/html/".to_string(),
                fallback: false,
                basic_auth: None,
                cache_type: CacheType::None,
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
        assert_eq!(
            Location {
                location: "/".to_string(),
                domain: None,
                alias: "/var/www/html/".to_string(),
                fallback: false,
                basic_auth: None,
                cache_type: CacheType::MustRevalidate,
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
        assert_eq!(
            Location {
                location: "/".to_string(),
                domain: Some("http://app:8000".to_string()),
                alias: "/".to_string(),
                fallback: true,
                basic_auth: None,
                cache_type: CacheType::None,
            }
            .render()
            .expect("failed to render location"),
            r#"  location / {
    proxy_pass http://app:8000/;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    try_files $uri $uri/ / =404;
  }"#
        );
    }

    #[test]
    fn test_location_3() {
        assert_eq!(
            Location {
                location: "/".to_string(),
                domain: None,
                alias: "/var/www/html/".to_string(),
                fallback: false,
                basic_auth: Some("/etc/nginx/conf.d/htpasswd".to_string()),
                cache_type: CacheType::None,
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
    fn test_server_0() {
        assert_eq!(
            Server {
                domain: None,
                port: Some(99),
                locations: vec![]
            }
            .render()
            .expect("failed to render location"),
            r#"server {
  listen 99;
}"#
        );
    }

    #[test]
    fn test_server_1() {
        assert_eq!(
            Server {
                domain: Some("foo.localhost".to_string()),
                port: None,
                locations: vec![]
            }
            .render()
            .expect("failed to render location"),
            r#"server {
  listen 80;
  server_name foo.localhost;
}"#
        );
    }
}
