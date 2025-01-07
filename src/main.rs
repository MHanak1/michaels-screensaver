#![windows_subsystem = "windows"]

use config::{Config, FileFormat};
use michaels_screensaver::{get_config, run, DEFAULT_CONFIG};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let mut config_path = dirs::config_dir().unwrap().to_path_buf();
    config_path.push("michaels-screensaver.toml");
    if !config_path.exists() {
        let file = File::create(config_path.clone());
        match file {
            Ok(mut file) => {
                file.write_all(DEFAULT_CONFIG).unwrap();
            }
            Err(e) => {
                panic!(
                    "Error creating config file at {}: {}",
                    config_path.display(),
                    e
                );
            }
        }
    }

    let config = get_config();

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "error");
    }
    env_logger::init();

    let config = config.try_deserialize::<HashMap<String, String>>().unwrap();

    println!("{:?}", config);

    pollster::block_on(run());
}
