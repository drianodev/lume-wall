use anyhow::Result;
use chrono::{Local, Utc};
use clap::{Parser, Subcommand};
use lumewall_backend::{DesktopBackend, GnomeBackend};
use lumewall_core::{calculate_solar_position, Config, Profile};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lumewall")]
#[command(author = "Adriano <drianodev>")]
#[command(version = "0.1.0")]
#[command(about = "Gerenciador unificado de papéis de parede dinâmicos e vídeo para Ubuntu / Linux", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Exibe o status atual do LumeWall, perfil ativo e informações solares
    Status,
    /// Lista todos os perfis configurados em config.toml
    List,
    /// Ativa um perfil específico e avisa o daemon
    SetProfile {
        /// Nome do perfil registrado no config.toml
        profile_name: String,
    },
    /// Aplica uma imagem avulsa diretamente como papel de parede (tema claro e escuro)
    Apply {
        /// Caminho para a imagem
        image_path: PathBuf,
    },
    /// Exibe os cálculos astronômicos atuais para as coordenadas configuradas
    Solar,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = Config::default_config_path()?;

    let config = if config_path.exists() {
        Config::load_from_file(&config_path)?
    } else {
        let scaffold = Config::create_default_scaffold();
        scaffold.save_to_file(&config_path)?;
        scaffold
    };

    let backend = GnomeBackend;

    match cli.command {
        Commands::Status => {
            println!("🌅 LumeWall Status");
            println!("--------------------------------------------------");
            println!("Configuração:       {}", config_path.display());
            println!("Perfil Ativo:       {}", config.general.active_profile);
            println!("Intervalo de Checagem: {}s", config.general.update_interval_seconds);

            let current_wall = backend.get_current_wallpaper().unwrap_or_else(|_| "Desconhecido".to_string());
            println!("Wallpaper Atual:    {}", current_wall);

            if let Some(profile) = config.profiles.get(&config.general.active_profile) {
                match profile {
                    Profile::Solar { description, frames } => {
                        println!("Tipo de Perfil:     Solar");
                        if let Some(desc) = description {
                            println!("Descrição:          {}", desc);
                        }
                        println!("Qtd de Frames:      {}", frames.len());
                    }
                    Profile::Time { description, frames } => {
                        println!("Tipo de Perfil:     Horário Fixo");
                        if let Some(desc) = description {
                            println!("Descrição:          {}", desc);
                        }
                        println!("Qtd de Frames:      {}", frames.len());
                    }
                    Profile::Video { description, video_path, .. } => {
                        println!("Tipo de Perfil:     Vídeo");
                        if let Some(desc) = description {
                            println!("Descrição:          {}", desc);
                        }
                        println!("Arquivo de Vídeo:   {}", video_path.display());
                    }
                }
            } else {
                println!("Tipo de Perfil:     [AVISO: Perfil '{}' não encontrado em config.toml]", config.general.active_profile);
            }
        }
        Commands::List => {
            println!("📋 Perfis Disponíveis no LumeWall:");
            println!("--------------------------------------------------");
            for (name, prof) in &config.profiles {
                let marker = if *name == config.general.active_profile { "* (ativo)" } else { "" };
                let ptype = match prof {
                    Profile::Solar { .. } => "Solar",
                    Profile::Time { .. } => "Horário",
                    Profile::Video { .. } => "Vídeo",
                };
                println!("• {:<20} [{:<8}] {}", name, ptype, marker);
            }
        }
        Commands::SetProfile { profile_name } => {
            if !config.profiles.contains_key(&profile_name) {
                eprintln!("Erro: Perfil '{}' não existe na configuração.", profile_name);
                std::process::exit(1);
            }

            let mut updated_config = config;
            updated_config.general.active_profile = profile_name.clone();
            updated_config.save_to_file(&config_path)?;
            println!("✅ Perfil ativo atualizado para: '{}'", profile_name);

            // Tenta enviar SIGHUP para o processo lumewalld se estiver rodando
            let _ = std::process::Command::new("pkill")
                .args(["-HUP", "lumewalld"])
                .status();
            println!("Notificação enviada ao lumewalld.");
        }
        Commands::Apply { image_path } => {
            if !image_path.exists() {
                eprintln!("Erro: O arquivo '{:?}' não foi encontrado.", image_path);
                std::process::exit(1);
            }
            backend.set_wallpaper(&image_path, None)?;
            println!("✅ Papel de parede aplicado: {:?}", image_path);
        }
        Commands::Solar => {
            let coords = config.location;
            let now_utc = Utc::now();
            let now_local = Local::now();
            let pos = calculate_solar_position(&coords, now_utc);

            println!("☀️  Dados Solares Astronômicos (NOAA SPA Offline)");
            println!("--------------------------------------------------");
            println!("Hora Local:         {}", now_local.format("%Y-%m-%d %H:%M:%S"));
            println!("Hora UTC:           {}", now_utc.format("%Y-%m-%d %H:%M:%S"));
            println!("Latitude:           {:.4}°", coords.latitude);
            println!("Longitude:          {:.4}°", coords.longitude);
            println!("Elevação (Altitude): {:.2}°", pos.altitude);
            println!("Azimute Solar:      {:.2}°", pos.azimuth);

            let status_str = if pos.altitude > 0.0 {
                "Dia (Sol acima do horizonte)"
            } else if pos.altitude > -6.0 {
                "Crepúsculo Civil"
            } else if pos.altitude > -12.0 {
                "Crepúsculo Náutico"
            } else if pos.altitude > -18.0 {
                "Crepúsculo Astronômico"
            } else {
                "Noite Fechada"
            };
            println!("Fase Solar:         {}", status_str);
        }
    }

    Ok(())
}
