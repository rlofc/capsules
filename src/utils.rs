use std::path::{Path, PathBuf};
use std::{env, fs};

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    capsule_volume_dir: Option<String>,
    capsule_home_dir: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CapsuleFile {
    pub blueprint: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let home_dir = env::home_dir().expect("Could not get home directory");
        let config_dir = home_dir.join(".config").join("capsules");
        let config_file = config_dir.join("capsules.toml");

        if let Ok(contents) = fs::read_to_string(&config_file) {
            if let Ok(cfg) = toml::from_str::<Config>(&contents) {
                return cfg;
            }
        }

        Config {
            capsule_volume_dir: None,
            capsule_home_dir: None,
        }
    }

    pub fn capsule_volume_dir(&self) -> &str {
        self.capsule_volume_dir.as_deref().unwrap_or("/files")
    }

    pub fn capsule_home_dir(&self) -> &str {
        self.capsule_home_dir.as_deref().unwrap_or("home")
    }
}

pub fn get_user_config_folder() -> PathBuf {
    let home_dir = env::home_dir().expect("Could not get home directory");
    home_dir.join(".config").join("capsules")
}

pub fn copy_directory(src: &str, dst: &str) -> std::io::Result<()> {
    let src_path = Path::new(src);
    let dst_path = Path::new(dst);

    if src_path.is_dir() {
        fs::create_dir_all(dst_path)?;
        for entry in fs::read_dir(src_path)? {
            let entry = entry?;
            let entry_path = entry.path();
            let new_path = dst_path.join(entry.file_name());

            if entry_path.is_dir() {
                copy_directory(entry_path.to_str().unwrap(), new_path.to_str().unwrap())?;
            } else {
                fs::copy(entry_path, new_path)?;
            }
        }
    }
    Ok(())
}
