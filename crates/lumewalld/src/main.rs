use anyhow::Result;
use chrono::Utc;
use lumewall_backend::desktop::{DesktopBackend, GnomeBackend};
use lumewall_core::{calculate_solar_position, Config, Profile, SolarFrame};
use std::sync::Arc;
use tracing::{error, info};

fn find_closest_solar_frame<'a>(frames: &'a [SolarFrame], target_alt: f64) -> Option<&'a SolarFrame> {
    frames.iter().min_by(|a, b| {
        let diff_a = (a.altitude - target_alt).abs();
        let diff_b = (b.altitude - target_alt).abs();
        diff_a.partial_cmp(&diff_b).unwrap_or(std::cmp::Ordering::Equal)
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Iniciando lumewalld (LumeWall Daemon)...");

    let config_path = Config::default_config_path()?;
    let config = if config_path.exists() {
        info!("Carregando configuração de {:?}", config_path);
        Config::load_from_file(&config_path)?
    } else {
        info!("Criando configuração padrão em {:?}", config_path);
        let scaffold = Config::create_default_scaffold();
        scaffold.save_to_file(&config_path)?;
        scaffold
    };

    let backend: Arc<dyn DesktopBackend> = Arc::new(GnomeBackend);

    info!("Perfil ativo: {}", config.general.active_profile);

    // Loop de scheduler principal (verifica e calcula o frame a cada ciclo)
    let profile = config.profiles.get(&config.general.active_profile);

    if let Some(Profile::Solar { frames, .. }) = profile {
        let coords = config.location;
        info!("Modo Solar ativado para coordenadas: lat={}, lon={}", coords.latitude, coords.longitude);

        let now = Utc::now();
        let pos = calculate_solar_position(&coords, now);
        info!("Posição Solar calculada: Altitude={:.2}°, Azimute={:.2}°", pos.altitude, pos.azimuth);

        if let Some(closest) = find_closest_solar_frame(frames, pos.altitude) {
            info!("Frame mais adequado selecionado: {:?}", closest.path);
            if closest.path.exists() {
                if let Err(e) = backend.set_wallpaper(&closest.path, closest.path_dark.as_deref()) {
                    error!("Erro ao definir wallpaper: {}", e);
                }
            } else {
                info!("Arquivo de wallpaper {:?} ainda não existe localmente (demo)", closest.path);
            }
        }
    }

    info!("lumewalld operacional.");
    Ok(())
}
