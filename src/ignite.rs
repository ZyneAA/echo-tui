use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use directories::ProjectDirs;

use crate::config::Config;

use super::config;

pub struct Paths {
    pub config: std::path::PathBuf,
    pub data: std::path::PathBuf,
}

impl Paths {
    fn init() -> Self {
        let proj = ProjectDirs::from("", "", "echo").expect("couldn't find dirs");

        let config = proj.config_dir();
        let data = proj.data_dir();

        for dir in [&config, &data] {
            fs::create_dir_all(dir).unwrap();
        }

        // configs
        let config_dir = config.join("config");
        fs::create_dir_all(&config_dir).unwrap();

        let config_file = config_dir.join("echo.toml");
        if !config_file.exists() {
            fs::write(&config_file, "").unwrap();
        }

        // data
        fs::create_dir_all(data.join("songs")).unwrap();
        fs::create_dir_all(data.join("playlists")).unwrap();

        Self {
            config: config.to_path_buf(),
            data: data.to_path_buf(),
        }
    }
}

pub fn engine() -> Result<(Config, PathBuf), Box<dyn std::error::Error>> {
    let paths = Paths::init();

    let config_file = paths.config.join("config/echo.toml");

    let mut config: String = String::new();
    File::open(config_file)?.read_to_string(&mut config)?;

    let config_vals: config::Config = toml::from_str(&config)?;
    for (name, style) in &config_vals.colors {
        println!("Name: {}", name);
        println!("  BG: {:?}", style.bg);
        println!("  FG: {:?}", style.fg);
    }

    let ok = (config_vals, paths.data);
    Ok(ok)
}


