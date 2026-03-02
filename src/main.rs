use clap::Parser;
use ee_nginx::{parse, output, Config as NginxConfig, CacheType, ParsedResult};
use serde::Serialize;
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

    /// Validate configuration only, do not output files
    #[arg(long)]
    validate: bool,

    /// Enable verbose output for validation
    #[arg(long)]
    verbose: bool,

    /// Output format: text, json, or yaml
    #[arg(long, default_value = "text", value_enum)]
    output_format: OutputFormat,

    /// Custom template directory (requires recompilation for changes)
    #[arg(long)]
    template_dir: Option<String>,

    /// Watch for file changes and auto-regenerate
    #[arg(short, long)]
    watch: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum OutputFormat {
    #[default]
    Text,
    Json,
    Yaml,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Yaml => write!(f, "yaml"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "yaml" => Ok(OutputFormat::Yaml),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }
}

impl Args {
    fn get_output_conf(&self) -> NginxConfig {
        let docker_mode = std::env::var("NGINX_IN_DOCKER")
            .ok()
            .unwrap_or("false".to_string())
            == "true";
        NginxConfig {
            docker_mode
        }
    }
    fn get_nginx_conf(&self) -> String {
        // Check for environment-specific configuration
        let env = std::env::var("NGINX_ENV").unwrap_or_default();
        
        if !env.is_empty() {
            let key = format!("NGINX_CONF_{}", env.to_uppercase());
            if let Ok(conf) = std::env::var(&key) {
                return conf;
            }
        }
        
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
    
    // If watch mode is enabled, enter watch loop
    if let Some(watch_path) = &args.watch {
        run_watch_mode(&args, watch_path, &conf, &nameserver, &hosts);
        return;
    }
    
    // Normal single-run mode
    run_single(&args, &conf, &nameserver, &hosts);
}

fn run_single(args: &Args, conf: &NginxConfig, nameserver: &str, hosts: &HashMap<String, IpAddr>) {
    let parse_result = parse(
        &PathBuf::from(&args.dst_dir),
        &args.get_nginx_conf(),
        conf,
        nameserver,
        hosts,
    );

    match parse_result {
        Ok(parsed_result) => {
            // Output format handling
            match args.output_format {
                OutputFormat::Json => {
                    let output = OutputResult::from(&parsed_result);
                    println!("{}", serde_json::to_string_pretty(&output).expect("failed to serialize to json"));
                    return;
                }
                OutputFormat::Yaml => {
                    let output = OutputResult::from(&parsed_result);
                    println!("{}", serde_yaml::to_string(&output).expect("failed to serialize to yaml"));
                    return;
                }
                OutputFormat::Text => {
                    // Continue with normal flow
                }
            }
            
            if args.validate {
                // Validation mode - print success message and details
                let server_count = parsed_result.server_map.len();
                let location_count: usize = parsed_result.server_map.values()
                    .map(|s| s.locations.len())
                    .sum();
                
                println!("✓ Valid configuration");
                println!("✓ {} server block(s)", server_count);
                println!("✓ {} location block(s)", location_count);
                
                if args.verbose {
                    println!("\nServers:");
                    for (domain, server) in &parsed_result.server_map {
                        println!("  - {}:{}", domain, server.port.unwrap_or(80));
                        for location in &server.locations {
                            println!("    {} -> {}", location.location, location.domain.as_ref().unwrap_or(&"static".to_string()));
                        }
                    }
                }
            } else {
                // Normal mode - output files
                output(&parsed_result).expect("output failed");
            }
        }
        Err(e) => {
            if args.validate {
                // Validation mode - print error message
                eprintln!("✗ Invalid configuration");
                eprintln!("✗ Error: {}", e);
                std::process::exit(1);
            } else {
                // Normal mode - panic
                panic!("parse failed: {}", e);
            }
        }
    }
}

fn run_watch_mode(args: &Args, watch_path: &str, conf: &NginxConfig, nameserver: &str, hosts: &HashMap<String, IpAddr>) {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;
    
    let (tx, rx) = channel();
    
    let mut watcher = RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default().with_poll_interval(Duration::from_secs(2)),
    ).expect("Failed to create watcher");
    
    let path_to_watch = PathBuf::from(watch_path);
    watcher.watch(&path_to_watch, RecursiveMode::Recursive)
        .expect("Failed to watch path");
    
    println!("Watching {} for changes...", watch_path);
    
    // Initial run
    run_single(args, conf, nameserver, hosts);
    
    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(event) => {
                if event.kind.is_modify() || event.kind.is_create() {
                    println!("\n--- File changed, regenerating ---");
                    run_single(args, conf, nameserver, hosts);
                    println!("--- Done ---\n");
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

// Serializable output structure for JSON/YAML
#[derive(Debug, Serialize)]
struct OutputResult {
    target_dir: String,
    servers: Vec<ServerOutput>,
}

#[derive(Debug, Serialize)]
struct ServerOutput {
    domain: String,
    port: Option<u16>,
    locations: Vec<LocationOutput>,
}

#[derive(Debug, Serialize)]
struct LocationOutput {
    location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    basic_auth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nameserver: Option<String>,
    #[serde(skip_serializing_if = "Not::is_false")]
    fallback: bool,
    #[serde(skip_serializing_if = "Not::is_false")]
    show_index: bool,
    #[serde(skip_serializing_if = "Not::is_false")]
    is_file: bool,
    #[serde(skip_serializing_if = "Not::is_false")]
    enable_sse: bool,
}

trait Not {
    fn is_false(&self) -> bool;
}

impl Not for bool {
    fn is_false(&self) -> bool {
        !*self
    }
}

impl From<&ParsedResult<'_>> for OutputResult {
    fn from(result: &ParsedResult<'_>) -> Self {
        let servers: Vec<ServerOutput> = result.server_map
            .iter()
            .map(|(domain, server)| {
                let locations: Vec<LocationOutput> = server.locations
                    .iter()
                    .map(|loc| {
                        let cache_type = match loc.cache_type {
                            CacheType::None => None,
                            CacheType::MustRevalidate => Some("must-revalidate".to_string()),
                            CacheType::Versioned => Some("versioned".to_string()),
                        };
                        LocationOutput {
                            location: loc.location.clone(),
                            domain: loc.domain.clone(),
                            alias: if loc.alias != "/" { Some(loc.alias.clone()) } else { None },
                            basic_auth: loc.basic_auth.clone(),
                            cache_type,
                            nameserver: Some(loc.nameserver.clone()),
                            fallback: loc.fallback,
                            show_index: loc.show_index,
                            is_file: loc.is_file,
                            enable_sse: loc.enable_sse,
                        }
                    })
                    .collect();
                ServerOutput {
                    domain: domain.clone(),
                    port: server.port,
                    locations,
                }
            })
            .collect();
        
        OutputResult {
            target_dir: result.target_dir.to_string_lossy().to_string(),
            servers,
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_parse_etc_hosts() {}
}
