use crate::screensaver::{BallColorMode, ScreenSaverType};
use crate::util::model::DDDModel;
use crate::{run_with_config, screensaver};
use config::Config;
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Add, AddAssign};
use std::process::exit;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_sys::window;
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

pub enum ConfigPresets {
    BallsInfection,
    BallsLava,
    BallsGasSimulation,
    BallsDVD,
    Colors,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Configurator {
    pub(crate) screensaver: screensaver::ScreenSaverType,
    pub(crate) fullscreen: bool,

    //Snow
    pub(crate) snowflake_count: usize,

    //Balls
    pub(crate) ball_count: usize,
    pub(crate) ball_speed: f32,
    pub(crate) ball_size: f32,
    pub(crate) color_mode: screensaver::BallColorMode,
    pub(crate) color: egui::Color32,
    pub(crate) show_density: bool,
    pub(crate) target_display_density: f64,
    pub(crate) region_size: f32,
    pub(crate) correct_ball_velocity: bool,

    //3D Model
    pub ddd_model: DDDModel,
    pub model_scale: f32,
    pub spin_speed: f32,
    pub bounce_speed: f32,
    pub bounce_height: f32,

    //Internal Use - Not Configurable
    pub(crate) preview_window: bool,
    pub should_reload: bool,
}

impl Configurator {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_config(&self) {
        let mut config_path = dirs::config_dir().unwrap().to_path_buf();
        config_path.push("michaels-screensaver.toml");
        let mut toml = File::open(&config_path).unwrap();
        let mut toml_string = String::new();
        toml.read_to_string(&mut toml_string).unwrap();
        use toml_edit::value;
        let mut doc = toml_edit::DocumentMut::from_str(toml_string.as_str()).unwrap();

        doc["screensaver"] = value(self.screensaver.to_string());
        doc["fullscreen"] = value(self.fullscreen);
        //Snow
        doc["snow"]["snowflake_count"] = value(self.snowflake_count as i64);
        //Balls
        doc["balls"]["speed"] = value(self.ball_speed as f64);
        doc["balls"]["count"] = value(self.ball_count as i64);
        doc["balls"]["size"] = value(self.ball_size as f64);
        doc["balls"]["color_mode"] = value(self.color_mode.to_string());
        doc["balls"]["show_density"] = value(self.show_density);
        doc["balls"]["target_display_density"] = value(self.target_display_density);
        doc["balls"]["color"] = value(self.color.to_hex()[0..7].to_string());
        doc["balls"]["region_size"] = value(self.region_size as f64);
        doc["balls"]["correct_ball_velocity"] = value(self.correct_ball_velocity);

        doc["3d_model"]["model"] = value(self.ddd_model.to_string());
        doc["3d_model"]["model_scale"] = value(self.model_scale as f64);
        doc["3d_model"]["spin_speed"] = value(self.spin_speed as f64);
        doc["3d_model"]["bounce_speed"] = value(self.bounce_speed as f64);
        doc["3d_model"]["bounce_height"] = value(self.bounce_height as f64);

        let mut toml = File::create(config_path).unwrap();
        toml.write_all(doc.to_string().as_bytes()).unwrap();
    }

    pub fn generate_url(&self) -> String {
        let dc = Self::default();

        #[cfg(not(target_arch = "wasm32"))]
        let mut url = String::from("https://mhanak.net/screensaver/");
        #[cfg(target_arch = "wasm32")]
        let mut url = window()
            .unwrap()
            .location()
            .to_string()
            .as_string()
            .unwrap();

        if url.contains("?") {
            url.truncate(url.find("?").unwrap());
            log::error!("{}", url);
        }

        url += format!("?screensaver={}", self.screensaver.to_string()).as_str();

        if dc.snowflake_count != self.snowflake_count {
            url += format!("&snowflake_count={}", self.snowflake_count).as_str()
        }

        if dc.ball_count != self.ball_count {
            url += format!("&count={}", self.ball_count).as_str()
        }
        if dc.ball_speed != self.ball_speed {
            url += format!("&speed={}", self.ball_speed).as_str()
        }
        if dc.ball_size != self.ball_size {
            url += format!("&size={}", self.ball_size).as_str()
        }
        if dc.color_mode != self.color_mode {
            url += format!("&color_mode={}", self.color_mode.to_string()).as_str()
        }
        if dc.color != self.color {
            url += format!("&color={}", self.color.to_hex()[0..7].replace("#", "%23")).as_str()
        }
        if dc.show_density != self.show_density {
            url += format!("&show_density={}", self.show_density).as_str()
        }
        if dc.target_display_density != self.target_display_density {
            url += format!("&target_display_density={}", self.target_display_density).as_str()
        }
        if dc.region_size != self.region_size {
            url += format!("&region_size={}", self.region_size).as_str()
        }
        if dc.correct_ball_velocity != self.correct_ball_velocity {
            url += format!("&correct_ball_velocity={}", self.correct_ball_velocity).as_str()
        }
        if dc.ddd_model != self.ddd_model {
            url += format!("&model={}", self.ddd_model.to_string()).as_str()
        }
        if dc.model_scale != self.model_scale {
            url += format!("&model_scale={}", self.model_scale.to_string()).as_str()
        }
        if dc.spin_speed != self.spin_speed {
            url += format!("&spin_speed={}", self.spin_speed).as_str()
        }
        if dc.bounce_speed != self.bounce_speed {
            url += format!("&bounce_speed={}", self.bounce_speed).as_str()
        }
        if dc.bounce_height != self.bounce_height {
            url += format!("&bounce_height={}", self.bounce_height).as_str()
        }

        url
    }

    pub fn from_config(config: Config) -> Self {
        let screensaver_name: String = config.get("screensaver").unwrap();
        let snow = config.get_table("snow").unwrap();
        let balls = config.get_table("balls").unwrap();
        let ddd_model = config.get_table("3d_model").unwrap();
        Self {
            screensaver: match screensaver_name.as_str() {
                "snow" => ScreenSaverType::Snow,
                "balls" => ScreenSaverType::Balls,
                "3d_model" => ScreenSaverType::DDDModel,
                _ => {
                    log::error!(
                        "Unknown screensaver: \"{}\", defaulting to \"snow\"",
                        screensaver_name
                    );
                    ScreenSaverType::Snow
                }
            },
            fullscreen: config.get("fullscreen").unwrap(),
            //Snow
            snowflake_count: snow
                .get("snowflake_count")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            //Balls
            ball_count: balls
                .get("count")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            ball_speed: config
                .get_table("balls")
                .unwrap()
                .get("speed")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            ball_size: balls
                .get("size")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            region_size: balls
                .get("region_size")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            correct_ball_velocity: balls
                .get("correct_ball_velocity")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            color_mode: match balls
                .get("color_mode")
                .unwrap()
                .clone()
                .try_deserialize::<Option<String>>()
                .unwrap()
            {
                Some(a) => match a.as_str() {
                    "random" => BallColorMode::Random,
                    "infection" => BallColorMode::Infection,
                    "color" => BallColorMode::Color,
                    "temperature" => BallColorMode::Temperature,
                    _ => BallColorMode::Random,
                },
                None => BallColorMode::Color,
            },
            color: {
                let color_hex: String = balls
                    .get("color")
                    .unwrap()
                    .clone()
                    .try_deserialize()
                    .unwrap();
                egui::Color32::from_hex(&color_hex).unwrap_or(egui::Color32::WHITE)
            },
            show_density: balls
                .get("show_density")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            target_display_density: balls
                .get("target_display_density")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            ddd_model: match ddd_model
                .get("model")
                .unwrap()
                .clone()
                .try_deserialize::<Option<String>>()
                .unwrap()
            {
                None => DDDModel::Apple,
                Some(a) => match a.as_str() {
                    "apple" => DDDModel::Apple,
                    "shark" => DDDModel::Shark,
                    "kim_kitsuragi" => DDDModel::KimKitsuragi,
                    _ => DDDModel::Apple,
                },
            },
            model_scale: ddd_model
                .get("model_scale")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            spin_speed: ddd_model
                .get("spin_speed")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            bounce_speed: ddd_model
                .get("bounce_speed")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            bounce_height: ddd_model
                .get("bounce_height")
                .unwrap()
                .clone()
                .try_deserialize()
                .unwrap(),
            preview_window: false,
            should_reload: false,
        }
    }

    pub fn from_preset(preset: ConfigPresets) -> Self {
        match preset {
            ConfigPresets::BallsInfection => Self {
                screensaver: ScreenSaverType::Balls,
                ball_count: 100,
                ball_speed: 0.2,
                ball_size: 0.2,
                color_mode: BallColorMode::Infection,
                ..Default::default()
            },
            ConfigPresets::BallsLava => Self {
                screensaver: ScreenSaverType::Balls,
                ball_count: 10000,
                ball_speed: 0.05,
                ball_size: 0.05,
                color_mode: BallColorMode::Temperature,
                show_density: true,
                region_size: 1.0,
                ..Default::default()
            },
            ConfigPresets::BallsGasSimulation => Self {
                screensaver: ScreenSaverType::Balls,
                ball_count: 50000,
                ball_speed: 0.1,
                ball_size: 0.03,
                color_mode: BallColorMode::Color,
                show_density: true,
                region_size: 0.5,
                correct_ball_velocity: false,
                ..Default::default()
            },
            ConfigPresets::BallsDVD => Self {
                screensaver: ScreenSaverType::Balls,
                ball_count: 1,
                ball_speed: 0.3,
                ball_size: 0.5,
                color_mode: BallColorMode::Random,
                ..Default::default()
            },
            ConfigPresets::Colors => Self {
                screensaver: ScreenSaverType::Balls,
                ball_count: 500,
                ball_speed: 0.2,
                ball_size: 0.1,
                color_mode: BallColorMode::Random,
                ..Default::default()
            },
        }
    }
}

impl Default for Configurator {
    fn default() -> Configurator {
        Configurator::from_config(crate::get_default_config())
    }
}

pub struct ConfigUI {
    pub configurator: Arc<Mutex<Configurator>>,
    color_picker_color: [f32; 3],
    clicked_gen_url: Instant,
    old_model: DDDModel,
}
impl ConfigUI {
    pub fn new(configurator: Arc<Mutex<Configurator>>) -> Self {
        Self {
            configurator,
            color_picker_color: [f32::NAN, f32::NAN, f32::NAN],
            clicked_gen_url: Instant::now()
                .checked_sub(Duration::from_secs(10))
                .unwrap_or(Instant::now()), //for some reason subtracting from an instant doesn't work on WASM
            old_model: DDDModel::Apple,
        }
    }
}

impl eframe::App for ConfigUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_arch = "wasm32")]
        let result = self.configurator.try_lock();
        #[cfg(not(target_arch = "wasm32"))]
        let result = self.configurator.lock();
        match result {
            Ok(mut configurator) => {
                if self.color_picker_color[0].is_nan() {
                    self.color_picker_color = [
                        configurator.color.r() as f32 / 255.0,
                        configurator.color.g() as f32 / 255.0,
                        configurator.color.b() as f32 / 255.0,
                    ];
                }
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Config");
                    egui::ComboBox::from_label("Screensaver")
                        .selected_text(format!("{:?}", configurator.screensaver))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut configurator.screensaver, ScreenSaverType::Snow, "Snow");
                            ui.selectable_value(&mut configurator.screensaver, ScreenSaverType::Balls, "Balls");
                            ui.selectable_value(&mut configurator.screensaver, ScreenSaverType::DDDModel, "3D Model");
                        });
                    ui.end_row();
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        match configurator.screensaver {
                            ScreenSaverType::Snow => {
                                ui.add(egui::Slider::new(&mut configurator.snowflake_count, 200..=20000).text("Snowflakes"));
                            }
                            ScreenSaverType::Balls => {
                                ui.add(egui::Slider::new(&mut configurator.ball_speed, 0.01..=1.0).text("Ball Speed"));
                                ui.end_row();
                                ui.horizontal(|ui| {
                                    let label = ui.label("Ball Count: ");
                                    ui.add(egui::DragValue::new(&mut configurator.ball_count).range(1..=100000)).labelled_by(label.id);
                                });
                                ui.end_row();
                                ui.add(egui::Slider::new(&mut configurator.ball_size, 0.02..=1.0).text("Ball Size"));
                                egui::ComboBox::from_label("Color Mode")
                                    .selected_text(format!("{:?}", configurator.color_mode))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut configurator.color_mode, BallColorMode::Random, "Random");
                                        ui.selectable_value(&mut configurator.color_mode, BallColorMode::Color, "Color");
                                        ui.selectable_value(&mut configurator.color_mode, BallColorMode::Infection, "Infection");
                                        ui.selectable_value(&mut configurator.color_mode, BallColorMode::Temperature, "Temperature");
                                    });
                                ui.end_row();
                                //don't ask me why it has to be this way
                                if configurator.color_mode == BallColorMode::Color {
                                    ui.color_edit_button_rgb(&mut self.color_picker_color);
                                    configurator.color = egui::Color32::from_rgb((self.color_picker_color[0] * 255.0) as u8, (self.color_picker_color[1] * 255.0) as u8, (self.color_picker_color[2] * 255.0) as u8);
                                    ui.end_row();
                                };
                                ui.add(egui::Checkbox::new(&mut configurator.show_density, "Show Density")).on_hover_text("change the opacity based on how many balls are in the surrounding regions and is influenced by their size.");
                                ui.end_row();
                                if configurator.show_density {
                                    ui.horizontal(|ui| {
                                        let label = ui.label("Target Density: ");
                                        ui.add(egui::DragValue::new(&mut configurator.target_display_density).range(1..=100)).labelled_by(label.id).on_hover_text("how many balls surrounding a given ball is needed for full opacity. if density display is all white, lower it. if it's too dark, make it higher");
                                        ui.end_row();
                                    });
                                }
                                ui.add(egui::Slider::new(&mut configurator.region_size, 0.5..=5.0).text("Region Size")).on_hover_text("For optimisation the space is split into chunks, and balls check for collisions in their chunk and those surrounding it. if you have a dense simulation, set it to 0.5, if you have a very sparse one set it to a higher value. if you don't know what this does keep it at 1.0.");
                                ui.end_row();
                                ui.add(egui::Checkbox::new(&mut configurator.correct_ball_velocity, "Correct Ball Velocity")).on_hover_text("Whether the speed of the balls should be adjusted if the average ball velocity is off");
                                ui.end_row();
                                ui.heading("Presets");
                                egui::ScrollArea::horizontal().show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("Infection").clicked() {
                                            *configurator = Configurator::from_preset(ConfigPresets::BallsInfection);
                                        };
                                        if ui.button("Lava").clicked() {
                                            *configurator = Configurator::from_preset(ConfigPresets::BallsLava);
                                        };
                                        if ui.button("Gas Simulation").clicked() {
                                            *configurator = Configurator::from_preset(ConfigPresets::BallsGasSimulation);
                                        }
                                        if ui.button("Just like the DVD logo").clicked() {
                                            *configurator = Configurator::from_preset(ConfigPresets::BallsDVD);
                                        }
                                        if ui.button("Colors!").clicked() {
                                            *configurator = Configurator::from_preset(ConfigPresets::Colors);
                                        }
                                    });
                                    ui.add_space(10.0);
                                });
                                ui.end_row();
                            }
                            ScreenSaverType::DDDModel => {
                                egui::ComboBox::from_label("Model")
                                    .selected_text(format!("{:?}", configurator.ddd_model))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut configurator.ddd_model, DDDModel::Apple, "Apple");
                                        ui.selectable_value(&mut configurator.ddd_model, DDDModel::Shark, "Shark");
                                        ui.selectable_value(&mut configurator.ddd_model, DDDModel::KimKitsuragi, "Kim Kitsuragi");
                                        //ui.selectable_value(&mut configurator.ddd_model, DDDModel::Custom, "Custom");
                                    });
                                if self.old_model != configurator.ddd_model {
                                    configurator.should_reload = true;
                                }
                                ui.add(egui::Slider::new(&mut configurator.model_scale, 0.1..=3.0).text("Model Size"));
                                ui.add(egui::Slider::new(&mut configurator.spin_speed, 0.0..=5.0).text("Spin Speed"));
                                ui.add(egui::Slider::new(&mut configurator.bounce_speed, 0.0..=5.0).text("Bounce Speed"));
                                ui.add(egui::Slider::new(&mut configurator.bounce_height, 0.0..=1.0).text("Bounce Height"));

                                self.old_model = configurator.ddd_model;
                            }
                        }
                        ui.separator();
                        ui.horizontal(|ui| {
                            #[cfg(not(target_arch = "wasm32"))]
                            if ui.add(egui::Button::new("Save and Exit")).clicked() {
                                configurator.save_config();
                                exit(0);
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            if ui.add(egui::Button::new("Exit without Saving")).clicked() {
                                exit(0);
                            }
                            if ui.add(egui::Button::new("Reset Settings")).clicked() {
                                *configurator = Configurator::default();
                                self.color_picker_color = [f32::INFINITY, f32::INFINITY, f32::INFINITY];
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            ui.separator();
                            #[cfg(not(target_arch = "wasm32"))]
                            if ui.add(egui::Button::new("Test")).clicked() {
                                configurator.preview_window = true;
                                let config = Arc::clone(&self.configurator);
                                thread::spawn(move || {
                                    pollster::block_on(run_with_config(config));
                                });
                            }
                            #[cfg(target_arch = "wasm32")]
                            {
                                let url_btn = ui.add(egui::Button::new("Generate URL"));
                                if url_btn.clicked() {
                                    self.clicked_gen_url = Instant::now();
                                    ctx.copy_text(configurator.generate_url());
                                }
                                if Instant::now().duration_since(self.clicked_gen_url) < Duration::from_secs(1) {
                                    url_btn.show_tooltip_text("Copied Link");
                                }
                            }
                        });
                        ui.end_row();
                    });
                });
            }
            Err(e) => {
                log::error!("{}", e);
            }
        }
    }
}
