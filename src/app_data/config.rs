use std::{fs::{self, File}, io::Write};

use serde::{Deserialize, Serialize};
use color_eyre::{Result, eyre::{Context}};
use serde_with::{serde_as, NoneAsEmptyString};

use crate::app_data;

const API_KEYS_FILE_NAME: &str = "config.toml";

#[serde_as]
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ApiKeys {
    #[serde_as(as = "NoneAsEmptyString")]
    pub zai: Option<String>,

    #[serde_as(as = "NoneAsEmptyString")]
    pub gemini: Option<String>,
}

pub fn read_config() -> Result<Config> {
    let config_file = fs::read_to_string(
        app_data::dirs().config().join(API_KEYS_FILE_NAME)
    )?;

    let toml: Config = toml::from_str(&config_file)
        .wrap_err("failed to parse toml")?;

    Ok(toml)
}

pub fn create_config() -> Result<Config> {
    fs::create_dir_all(app_data::dirs().config())?;

    let mut config = File::create_new(app_data::dirs().config().join(API_KEYS_FILE_NAME))?;
    config.write_all(b"[api_keys]\nzai=\"\"\ngemini=\"\"")?;

    Ok(Config::default())
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Config {
    pub api_keys: ApiKeys,
}
