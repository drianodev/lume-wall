use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use tracing::info;

pub trait DesktopBackend: Send + Sync {
    fn set_wallpaper(&self, light_path: &Path, dark_path: Option<&Path>) -> Result<()>;
    fn get_current_wallpaper(&self) -> Result<String>;
}

pub struct GnomeBackend;

impl DesktopBackend for GnomeBackend {
    fn set_wallpaper(&self, light_path: &Path, dark_path: Option<&Path>) -> Result<()> {
        let light_path_canonical = light_path.canonicalize().unwrap_or(light_path.to_path_buf());
        let light_uri = format!("file://{}", light_path_canonical.display());
        let dark_uri = match dark_path {
            Some(p) => format!("file://{}", p.canonicalize().unwrap_or(p.to_path_buf()).display()),
            None => light_uri.clone(),
        };

        info!("Aplicando wallpaper GNOME: light={}", light_uri);

        // Define tema claro
        let status_light = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.background", "picture-uri", &light_uri])
            .status()
            .context("Falha ao executar gsettings para picture-uri")?;

        if !status_light.success() {
            anyhow::bail!("gsettings retornou erro ao definir picture-uri");
        }

        // Define tema escuro
        let status_dark = Command::new("gsettings")
            .args(["set", "org.gnome.desktop.background", "picture-uri-dark", &dark_uri])
            .status()
            .context("Falha ao executar gsettings para picture-uri-dark")?;

        if !status_dark.success() {
            anyhow::bail!("gsettings retornou erro ao definir picture-uri-dark");
        }

        Ok(())
    }

    fn get_current_wallpaper(&self) -> Result<String> {
        let output = Command::new("gsettings")
            .args(["get", "org.gnome.desktop.background", "picture-uri"])
            .output()
            .context("Falha ao consultar gsettings picture-uri")?;

        let uri = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(uri)
    }
}
