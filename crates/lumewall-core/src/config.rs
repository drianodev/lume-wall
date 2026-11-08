use crate::solar::SolarCoordinates;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    #[serde(default)]
    pub location: SolarCoordinates,
    #[serde(default)]
    pub video: VideoConfig,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub active_profile: String,
    #[serde(default = "default_true")]
    pub battery_saver: bool,
    #[serde(default = "default_true")]
    pub pause_on_fullscreen: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_true() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    #[serde(default = "default_hwdec")]
    pub mpv_hwdec: String,
    #[serde(default = "default_fps")]
    pub fps_limit: u32,
    #[serde(default)]
    pub volume: u32,
}

fn default_hwdec() -> String {
    "auto-safe".to_string()
}

fn default_fps() -> u32 {
    30
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            mpv_hwdec: default_hwdec(),
            fps_limit: default_fps(),
            volume: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Profile {
    #[serde(rename = "solar")]
    Solar {
        description: Option<String>,
        frames: Vec<SolarFrame>,
    },
    #[serde(rename = "time")]
    Time {
        description: Option<String>,
        frames: Vec<TimeFrame>,
    },
    #[serde(rename = "video")]
    Video {
        description: Option<String>,
        video_path: PathBuf,
        #[serde(default = "default_true")]
        loop_video: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarFrame {
    pub azimuth: f64,
    pub altitude: f64,
    pub path: PathBuf,
    pub path_dark: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeFrame {
    /// Horário no formato "HH:MM" (24h)
    pub time: String,
    pub path: PathBuf,
    pub path_dark: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProfileType {
    Solar,
    Time,
    Video,
}

impl Config {
    pub fn default_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Não foi possível encontrar o config_dir")?;
        Ok(config_dir.join("lumewall").join("config.toml"))
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Retorna uma configuração padrão com perfil solar de demonstração
    pub fn create_default_scaffold() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert(
            "mojave-demo".to_string(),
            Profile::Solar {
                description: Some("Perfil solar padrão de demonstração".to_string()),
                frames: vec![
                    SolarFrame {
                        azimuth: 90.0,
                        altitude: -6.0,
                        path: PathBuf::from("/usr/share/backgrounds/dawn.jpg"),
                        path_dark: None,
                    },
                    SolarFrame {
                        azimuth: 180.0,
                        altitude: 60.0,
                        path: PathBuf::from("/usr/share/backgrounds/day.jpg"),
                        path_dark: None,
                    },
                    SolarFrame {
                        azimuth: 270.0,
                        altitude: -6.0,
                        path: PathBuf::from("/usr/share/backgrounds/dusk.jpg"),
                        path_dark: None,
                    },
                    SolarFrame {
                        azimuth: 0.0,
                        altitude: -45.0,
                        path: PathBuf::from("/usr/share/backgrounds/night.jpg"),
                        path_dark: None,
                    },
                ],
            },
        );

        Self {
            general: GeneralConfig {
                active_profile: "mojave-demo".to_string(),
                battery_saver: true,
                pause_on_fullscreen: true,
                log_level: "info".to_string(),
            },
            location: SolarCoordinates::default(),
            video: VideoConfig::default(),
            profiles,
        }
    }
}
