use anyhow::Result;
use chrono::{Local, Timelike, Utc};
use lumewall_backend::{DesktopBackend, GnomeBackend, MpvManager};
use lumewall_core::{calculate_solar_position, Config, Profile, SolarFrame, TimeFrame};
use std::path::PathBuf;
use std::time::Duration;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info, warn};

fn find_closest_solar_frame<'a>(frames: &'a [SolarFrame], target_alt: f64) -> Option<&'a SolarFrame> {
    frames.iter().min_by(|a, b| {
        let diff_a = (a.altitude - target_alt).abs();
        let diff_b = (b.altitude - target_alt).abs();
        diff_a.partial_cmp(&diff_b).unwrap_or(std::cmp::Ordering::Equal)
    })
}

fn parse_time_to_minutes(time_str: &str) -> Option<u32> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() == 2 {
        let h: u32 = parts[0].parse().ok()?;
        let m: u32 = parts[1].parse().ok()?;
        Some(h * 60 + m)
    } else {
        None
    }
}

fn find_active_time_frame<'a>(frames: &'a [TimeFrame], current_minutes: u32) -> Option<&'a TimeFrame> {
    if frames.is_empty() {
        return None;
    }

    let mut sorted_frames: Vec<(u32, &TimeFrame)> = frames
        .iter()
        .filter_map(|f| parse_time_to_minutes(&f.time).map(|m| (m, f)))
        .collect();

    sorted_frames.sort_by_key(|(m, _)| *m);

    let mut active = sorted_frames.last().map(|(_, f)| *f);
    for (m, f) in &sorted_frames {
        if current_minutes >= *m {
            active = Some(f);
        } else {
            break;
        }
    }
    active
}

async fn evaluate_and_apply(
    config: &Config,
    backend: &dyn DesktopBackend,
    mpv: &mut MpvManager,
    last_applied: &mut Option<PathBuf>,
) -> Result<()> {
    let profile = match config.profiles.get(&config.general.active_profile) {
        Some(p) => p,
        None => {
            warn!("Perfil ativo '{}' não encontrado", config.general.active_profile);
            return Ok(());
        }
    };

    match profile {
        Profile::Solar { frames, .. } => {
            let coords = config.location;
            let now = Utc::now();
            let pos = calculate_solar_position(&coords, now);

            if let Some(closest) = find_closest_solar_frame(frames, pos.altitude) {
                if last_applied.as_ref() != Some(&closest.path) {
                    if closest.path.exists() {
                        info!(
                            "Aplicando frame solar: {:?} (Sol: Alt={:.2}°, Az={:.2}°)",
                            closest.path, pos.altitude, pos.azimuth
                        );
                        backend.set_wallpaper(&closest.path, closest.path_dark.as_deref())?;
                        *last_applied = Some(closest.path.clone());
                    } else {
                        warn!("Frame solar indicado não existe no disco: {:?}", closest.path);
                    }
                }
            }
        }
        Profile::Time { frames, .. } => {
            let local_now = Local::now();
            let current_minutes = local_now.hour() * 60 + local_now.minute();

            if let Some(frame) = find_active_time_frame(frames, current_minutes) {
                if last_applied.as_ref() != Some(&frame.path) {
                    if frame.path.exists() {
                        info!("Aplicando frame de horário ({}) : {:?}", frame.time, frame.path);
                        backend.set_wallpaper(&frame.path, frame.path_dark.as_deref())?;
                        *last_applied = Some(frame.path.clone());
                    } else {
                        warn!("Frame de horário indicado não existe no disco: {:?}", frame.path);
                    }
                }
            }
        }
        Profile::Video { video_path, .. } => {
            if last_applied.as_ref() != Some(video_path) {
                if video_path.exists() {
                    info!("Iniciando vídeo wallpaper com mpv: {:?}", video_path);
                    mpv.start(
                        video_path,
                        &config.video.mpv_hwdec,
                        config.video.fps_limit,
                        config.video.volume,
                    )?;
                    *last_applied = Some(video_path.clone());
                } else {
                    warn!("Arquivo de vídeo indicado não existe: {:?}", video_path);
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("Iniciando lumewalld (LumeWall Daemon)...");

    let config_path = Config::default_config_path()?;
    let mut config = if config_path.exists() {
        info!("Carregando configuração de {:?}", config_path);
        Config::load_from_file(&config_path)?
    } else {
        info!("Criando configuração padrão em {:?}", config_path);
        let scaffold = Config::create_default_scaffold();
        scaffold.save_to_file(&config_path)?;
        scaffold
    };

    let backend = GnomeBackend;
    let mut mpv = MpvManager::new(&config.video.socket_path);
    let mut last_applied: Option<PathBuf> = None;

    // SIGHUP para recarga dinâmica de configuração
    let mut sighup = signal(SignalKind::hangup())?;
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    info!("lumewalld ativo com perfil '{}'", config.general.active_profile);

    // Avaliação inicial
    if let Err(e) = evaluate_and_apply(&config, &backend, &mut mpv, &mut last_applied).await {
        error!("Erro na aplicação inicial: {}", e);
    }

    let interval_secs = config.general.update_interval_seconds.max(5);
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = evaluate_and_apply(&config, &backend, &mut mpv, &mut last_applied).await {
                    error!("Erro no ciclo do scheduler: {}", e);
                }
            }
            _ = sighup.recv() => {
                info!("Sinal SIGHUP recebido. Recarregando configuração...");
                match Config::load_from_file(&config_path) {
                    Ok(new_cfg) => {
                        config = new_cfg;
                        last_applied = None; // Força reavaliação
                        info!("Configuração recarregada com sucesso. Novo perfil: {}", config.general.active_profile);
                        let _ = evaluate_and_apply(&config, &backend, &mut mpv, &mut last_applied).await;
                    }
                    Err(e) => {
                        error!("Falha ao recarregar config.toml: {}", e);
                    }
                }
            }
            _ = sigterm.recv() => {
                info!("Sinal SIGTERM recebido. Encerrando lumewalld...");
                break;
            }
            _ = sigint.recv() => {
                info!("Sinal SIGINT (Ctrl+C) recebido. Encerrando lumewalld...");
                break;
            }
        }
    }

    mpv.stop();
    info!("lumewalld finalizado.");
    Ok(())
}
