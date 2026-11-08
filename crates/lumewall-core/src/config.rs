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
    pub storage: StorageConfig,
    #[serde(default)]
    pub video: VideoConfig,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub active_profile: String,
    #[serde(default = "default_interval")]
    pub update_interval_seconds: u64,
    #[serde(default = "default_true")]
    pub battery_saver: bool,
    #[serde(default = "default_true")]
    pub pause_on_fullscreen: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_interval() -> u64 {
    60
}

fn default_true() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub wallpapers_dir: PathBuf,
    pub videos_dir: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/drianodev"));
        Self {
            wallpapers_dir: home.join("Pictures/LumeWall/Wallpapers"),
            videos_dir: home.join("Videos/LumeWall"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    #[serde(default = "default_hwdec")]
    pub mpv_hwdec: String,
    #[serde(default = "default_fps")]
    pub fps_limit: u32,
    #[serde(default)]
    pub volume: u32,
    #[serde(default = "default_socket_path")]
    pub socket_path: PathBuf,
}

fn default_hwdec() -> String {
    "auto-safe".to_string()
}

fn default_fps() -> u32 {
    30
}

fn default_socket_path() -> PathBuf {
    PathBuf::from("/tmp/lumewall-mpv.sock")
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            mpv_hwdec: default_hwdec(),
            fps_limit: default_fps(),
            volume: 0,
            socket_path: default_socket_path(),
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
    pub time: String,
    pub path: PathBuf,
    pub path_dark: Option<PathBuf>,
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

    pub fn ensure_storage_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.storage.wallpapers_dir)?;
        fs::create_dir_all(&self.storage.videos_dir)?;
        Ok(())
    }

    pub fn create_default_scaffold() -> Self {
        let storage = StorageConfig::default();
        let _ = fs::create_dir_all(&storage.wallpapers_dir);
        let _ = fs::create_dir_all(&storage.videos_dir);

        let mut profiles = HashMap::new();
        profiles.insert(
            "solar-local".to_string(),
            Profile::Solar {
                description: Some("Ciclo solar automático com imagens locais".to_string()),
                frames: vec![
                    SolarFrame {
                        azimuth: 90.0,
                        altitude: -2.0,
                        path: storage.wallpapers_dir.join("dawn.png"),
                        path_dark: None,
                    },
                    SolarFrame {
                        azimuth: 180.0,
                        altitude: 50.0,
                        path: storage.wallpapers_dir.join("day.png"),
                        path_dark: None,
                    },
                    SolarFrame {
                        azimuth: 270.0,
                        altitude: -5.0,
                        path: storage.wallpapers_dir.join("night.jpg"),
                        path_dark: None,
                    },
                    SolarFrame {
                        azimuth: 0.0,
                        altitude: -40.0,
                        path: storage.wallpapers_dir.join("night.jpg"),
                        path_dark: None,
                    },
                ],
            },
        );

        Self {
            general: GeneralConfig {
                active_profile: "solar-local".to_string(),
                update_interval_seconds: 30,
                battery_saver: true,
                pause_on_fullscreen: true,
                log_level: "info".to_string(),
            },
            location: SolarCoordinates::default(),
            storage,
            video: VideoConfig::default(),
            profiles,
        }
    }
}
