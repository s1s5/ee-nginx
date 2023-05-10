use clap::Parser;
use ee_nginx::{output, parse};
use std::path::PathBuf;

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
    fn get_conf(&self) -> String {
        if let Some(conf_str) = &self.conf_str {
            conf_str.clone()
        } else if let Some(conf_file) = &self.conf_file {
            std::fs::read_to_string(conf_file).expect("failed to read file")
        } else {
            std::env::var(&self.env_var)
                .ok()
                .expect("could not extract env value")
        }
    }
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    let parsed_result =
        parse(&PathBuf::from(&args.dst_dir), &args.get_conf()).expect("parse failed");

    output(&parsed_result).expect("output failed");
}
