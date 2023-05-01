use std::{io::Write, path::Path};

use askama::Template;
use base64::{engine::general_purpose, Engine};
use openssl::hash::{Hasher, MessageDigest};

use crate::{error::CustomError, utils::get_basic_auth_file_path, ParsedResult};

fn create_htpasswd(user: &str, password: &str) -> String {
    let mut hasher = Hasher::new(MessageDigest::sha1()).unwrap();
    hasher.update(password.as_bytes()).unwrap();
    let digest = hasher.finish().unwrap();

    let hashed = general_purpose::STANDARD.encode(digest);

    format!("{}:{{SHA}}{}", user, hashed)
}

fn write_to_file(dst_path: &Path, content: &str) -> Result<(), CustomError> {
    if dst_path.to_str().unwrap().starts_with("-/") {
        println!("----- {} -----", &dst_path.to_str().unwrap()[2..]);
        println!("{}", content);
    } else {
        let mut file = std::fs::File::create(&dst_path).map_err(|e| {
            CustomError::new(format!("failed to crete file. {:?}, {:?}", dst_path, e))
        })?;
        file.write_all(content.as_bytes()).map_err(|e| {
            CustomError::new(format!(
                "write_all failed for path {:?}, Error:{:?}",
                dst_path, e
            ))
        })?;
    }
    Ok(())
}

pub fn output(parsed_result: &ParsedResult) -> Result<(), CustomError> {
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
