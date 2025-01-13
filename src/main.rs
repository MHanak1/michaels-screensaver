#![windows_subsystem = "windows"]
#![cfg(not(target_arch = "wasm32"))]

use eframe::{HardwareAcceleration, Renderer};
use michaels_screensaver::configurator::{ConfigUI, Configurator};
use michaels_screensaver::{get_config, DEFAULT_CONFIG};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::{env, process};
use std::sync::{Arc, Mutex};

fn main() {
    let args: Vec<String> = env::args().collect();
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

    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "error");
    }
    env_logger::init();


    let config_app = ConfigUI{
        configurator: Arc::new(Mutex::new(Configurator::from_config(get_config()))),
    };

    //https://stackoverflow.com/questions/5165133/how-can-i-write-a-screen-saver-for-windows-in-c
    if cfg!(target_os = "windows") {
        if args.contains(&"/p".to_string()) || args.contains(&"\\p".to_string()) {
            process::exit(0);
        } else if args.contains(&"/s".to_string()) || args.contains(&"\\s".to_string()) {
            pollster::block_on(michaels_screensaver::run());
        } else {
            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 400.0]),
                hardware_acceleration: HardwareAcceleration::Off,
                renderer: Renderer::Wgpu,
                ..Default::default()
            };
            eframe::run_native(
                "My egui App",
                options,
                Box::new(|_cc| Ok(Box::new(config_app))),
            )
            .expect("TODO: panic message");
        }
    } else {
        if args.contains(&"-c".to_string()) || args.contains(&"--config".to_string()) {
            let options = eframe::NativeOptions {
                viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 400.0]),
                ..Default::default()
            };
            eframe::run_native(
                "Screensaver Config",
                options,
                Box::new(|_cc| Ok(Box::new(config_app))),
            )
            .expect("eframe brokey");
        } else {
            pollster::block_on(michaels_screensaver::run());
        }
    }
}
