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
    build:
      context: ..
    environment:
      NGINX_CONF: |
        / > /mnt/root/
        /app/ > http://app:8000/
        /static > /mnt/static/?versioned
        http://user:password@*/secret > /mnt/secret/
        http://hoge.localhost/ > /mnt/hoge/
        http://hoge.localhost/static > /mnt/static/?must-revalidate
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
