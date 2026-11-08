pub mod config;
pub mod solar;

pub use config::{Config, Profile, SolarFrame, StorageConfig, TimeFrame};
pub use solar::{calculate_solar_position, SolarCoordinates, SolarPosition};
