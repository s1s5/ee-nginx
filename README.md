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


# development
- docker buildx build --platform linux/amd64,linux/arm64 -f multi-platform.Dockerfile -t s1s5/ee-nginx .
