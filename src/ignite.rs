use std::{
    fs::{self, File},
    io::{self, Read},
    path::PathBuf,
};

use directories::ProjectDirs;

use crate::config::Config;

use super::config;

pub struct Paths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub songs: PathBuf,
    pub playlists: PathBuf
}

impl Paths {
    fn init() -> io::Result<Self> {
        let proj = ProjectDirs::from("", "", "echo").ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "couldn't find dirs"))?;

        let config = proj.config_dir();
        let data = proj.data_dir();

        for dir in [&config, &data] {
            fs::create_dir_all(dir)?;
        }

        // configs
        let config_dir = config.join("config");
        fs::create_dir_all(&config_dir)?;

        let config_file = config_dir.join("echo.toml");
        if !config_file.exists() {
            fs::write(&config_file, "")?;
        }

        // data
        fs::create_dir_all(data.join("songs"))?;
        fs::create_dir_all(data.join("playlists"))?;
        let songs = data.join("songs");
        let playlists = data.join("playlists");

        Ok(Self {
            config: config.to_path_buf(),
            data: data.to_path_buf(),
            songs,
            playlists
        })
    }
}

pub fn engine() -> Result<(Config, Paths), Box<dyn std::error::Error>> {
    let paths = Paths::init()?;

    let config_file = paths.config.join("config/echo.toml");

    let mut config: String = String::new();
    File::open(config_file)?.read_to_string(&mut config)?;

    let config_vals: config::Config = toml::from_str(&config)?;

    println!("{:?}", config_vals.colors);

    let ok = (config_vals, paths);
    Ok(ok)
}
