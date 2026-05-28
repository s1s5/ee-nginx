# ee-nginx
Easy configuration of nginx using environment variables.

## Usage
### in Docker
```shell
$ docker run -e NGINX_CONF="/>/var/www/html/;/static/>/mnt/static" s1s5/ee-nginx
```

### with docker compose
```yaml
version: '3'

services:
  nginx:
    environment:
      NGINX_CONF: |
        / > /mnt/root/                    # http://nginx/a.jpg -> /mnt/root/a.jpg  (no cache)
        /app/ > http://app:8000/          # http://nginx/app/profile/ -> http://app:8000/profile/
        /static > /mnt/static/?versioned  # cached, no validation
        http://user:password@*/secret > /mnt/secret/  # add basic auth
        http://hoge.localhost/ > /mnt/hoge/           # specific host routing
        http://hoge.localhost/static > /mnt/static/?must-revalidate  # cached, always check modification
```

## Run Example
```shell
$ cd example
$ docker compose up --build
```

## CLI Options

```
ee-nginx [OPTIONS]
  -c, --conf-str <CONF_STR>      Configuration string directly
      --conf-file <CONF_FILE>    Path to configuration file
  -e, --env-var <ENV_VAR>        Environment variable name (default: NGINX_CONF)
  -d, --dst-dir <DST_DIR>        Output directory (default: /etc/nginx/conf.d)
      --validate                 Validate configuration only, do not output files
      --verbose                  Enable verbose output for validation
      --output-format <FORMAT>   Output format: text, json, or yaml (default: text)
      --template-dir <DIR>        Custom template directory
  -w, --watch <PATH>             Watch for file changes and auto-regenerate
```

### --validate
Validate configuration without generating output files. Useful for CI/CD pipelines.

```shell
# Validate configuration
$ ee-nginx --conf-file nginx.conf --validate
✓ Valid configuration
✓ 3 server block(s)
✓ 5 location block(s)

# Verbose mode for detailed output
$ ee-nginx --conf-file nginx.conf --validate --verbose
✓ Valid configuration
✓ 3 server block(s)
✓ 5 location block(s)

Servers:
  - *:80
    / -> static
    /app -> http://app:8000/
  - example.com:443
    / -> /var/www/html/
```

### --output-format
Choose output format: `text` (nginx.conf), `json`, or `yaml`.

```shell
# JSON output
$ ee-nginx --conf-file nginx.conf --output-format json
{
  "target_dir": "/etc/nginx/conf.d",
  "servers": [
    {
      "domain": "*",
      "port": 80,
      "locations": [
        {
          "location": "/",
          "alias": "/var/www/html/",
          "cache_type": "versioned"
        }
      ]
    }
  ]
}

# YAML output
$ ee-nginx --conf-file nginx.conf --output-format yaml
target_dir: /etc/nginx/conf.d
servers:
  - domain: "*"
    port: 80
    locations:
      - location: /
        alias: /var/www/html/
        cache_type: versioned
```

### --template-dir
Use custom template directory instead of the default `templates/`.

```shell
$ ee-nginx --conf-file nginx.conf --template-dir /path/to/custom/templates
```

### --watch / -w
Watch configuration file or environment variable for changes and auto-regenerate output. Useful during development.

```shell
$ ee-nginx --watch nginx.conf
Watching nginx.conf for changes...
✓ Valid configuration
✓ 1 server block(s)
✓ 1 location block(s)

--- File changed, regenerating ---
✓ Valid configuration
✓ 1 server block(s)
✓ 1 location block(s)
--- Done ---

--- File changed, regenerating ---
✗ Invalid configuration
✗ Error: failed to parse "http://*/"
  → Missing target path
--- Done ---
```

### Environment-specific Configuration
Switch between different configurations using `NGINX_ENV`:

```shell
# Set environment to dev, prod, etc.
$ NGINX_ENV=dev ee-nginx --conf-file nginx.conf

# Uses NGINX_CONF_DEV environment variable when NGINX_ENV=dev
# Uses NGINX_CONF_PROD environment variable when NGINX_ENV=prod
```

```yaml
# docker-compose.yml
services:
  nginx-dev:
    environment:
      NGINX_ENV: dev
      NGINX_CONF_DEV: / > /mnt/dev/

  nginx-prod:
    environment:
      NGINX_ENV: prod
      NGINX_CONF_PROD: / > /mnt/prod/
```

## Features
- directory alias(root)
```
/ > /var/www/html/
```
It must start with a '/' and usually needs a trailing '/' almost every time.

- cache control
```
none -> no-cache
?must-revalidate -> no-store
?versioned -> max-age: 1year
```

- basic authorization
```
http://user:password@*/secret > /mnt/secret/
```
It must start with `http://`. and use '*' for default domain.

- for SPA
```
/ > /?fallback
```
will contains following settings
```
try_files $uri $uri/ / =404;
```

- show index
```
/ > /?index
```


## Build

### Multi-platform (amd64 + arm64)
```bash
docker buildx build --platform linux/amd64,linux/arm64 -f multi-platform.Dockerfile -t s1s5/ee-nginx .
```

### Single platform (amd64 only)
```bash
docker build -f Dockerfile -t s1s5/ee-nginx .
```

### Publish to Docker Hub
```bash
docker buildx build --platform linux/amd64,linux/arm64 -f multi-platform.Dockerfile -t s1s5/ee-nginx --push .
```
