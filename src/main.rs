use clap::Parser;
use ee_nginx::{output, parse, Config};
use std::{collections::HashMap, io::BufRead, net::IpAddr, path::PathBuf, str::FromStr};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    conf_str: Option<String>,

    #[arg(long)]
    conf_file: Option<String>,

    #[arg(short, long, default_value = "NGINX_CONF")]
    env_var: String,

    #[arg(short, long, default_value = "/etc/nginx/conf.d")]
    dst_dir: String,
}

impl Args {
    fn get_output_conf(&self) -> Config {
        let docker_mode = std::env::var("NGINX_IN_DOCKER")
            .ok()
            .unwrap_or("false".to_string())
            == "true";
        Config {
            docker_mode
        }
    }
    fn get_nginx_conf(&self) -> String {
        if let Some(conf_str) = &self.conf_str {
            conf_str.clone()
        } else if let Some(conf_file) = &self.conf_file {
            std::fs::read_to_string(conf_file).expect("failed to read file")
        } else {
            std::env::var(&self.env_var).expect("could not extract env value")
        }
    }
}

fn extract_nameserver_from_resolv_conf() -> std::io::Result<String> {
    let file = std::fs::File::open("/etc/resolv.conf")?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines() {
        let line = line?.trim().to_string();
        if line.starts_with('#') {
            continue;
        }
        let tokens: Vec<_> = line.split(char::is_whitespace).collect();
        if tokens.len() >= 2 && tokens[0] == "nameserver" {
            return Ok(tokens[1].to_string());
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "nameserver line not found",
    ))
}

fn extract_etc_hosts() -> std::io::Result<HashMap<String, IpAddr>> {
    let file = std::fs::File::open("/etc/hosts")?;
    let reader = std::io::BufReader::new(file);
    let mut hosts = HashMap::new();
    for line in reader.lines() {
        let line = line?.trim().to_string();
        if line.starts_with('#') {
            continue;
        }
        let tokens: Vec<_> = line.split(char::is_whitespace).collect();
        if tokens.len() == 2 {
            if let Ok(ipaddr) = IpAddr::from_str(tokens[0]) {
                let hostname = tokens[1];
                hosts.insert(hostname.to_string(), ipaddr);
            }
        }
    }
    Ok(hosts)
}

fn main() {
    env_logger::init();

    let args = Args::parse();
    let conf = args.get_output_conf();
    let nameserver = extract_nameserver_from_resolv_conf().unwrap_or("127.0.0.53".to_string());
    let hosts = extract_etc_hosts().unwrap_or_default();
    let parsed_result = parse(
        &PathBuf::from(&args.dst_dir),
        &args.get_nginx_conf(),
        &conf,
        &nameserver,
        &hosts,
    )
    .expect("parse failed");

    output(&parsed_result).expect("output failed");
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_parse_etc_hosts() {}
}
