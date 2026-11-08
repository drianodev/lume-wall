use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, info};

pub struct MpvManager {
    socket_path: PathBuf,
    process: Option<Child>,
}

impl MpvManager {
    pub fn new<P: AsRef<Path>>(socket_path: P) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
            process: None,
        }
    }

    /// Inicia o processo mpv em background com socket IPC pronto para receber streams/vídeos
    pub fn start(&mut self, video_path: &Path, hwdec: &str, fps: u32, volume: u32) -> Result<()> {
        self.stop();

        // Remove socket antigo se existir
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }

        let socket_str = format!("--input-ipc-server={}", self.socket_path.display());
        let hwdec_str = format!("--hwdec={}", hwdec);
        let fps_str = format!("--fps={}", fps);
        let volume_str = format!("--volume={}", volume);

        info!("Iniciando subprocesso mpv para vídeo wallpaper...");
        let child = Command::new("mpv")
            .args([
                &socket_str,
                &hwdec_str,
                &fps_str,
                &volume_str,
                "--loop-file=inf",
                "--no-osc",
                "--no-osd-bar",
                "--no-input-default-bindings",
                "--border=no",
                "--idle=yes",
                video_path.to_str().unwrap_or(""),
            ])
            .spawn()
            .context("Falha ao spawnar processo mpv")?;

        self.process = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.process.take() {
            info!("Finalizando processo mpv...");
            let _ = child.kill();
            let _ = child.wait();
        }
        if self.socket_path.exists() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
    }

    pub async fn send_command(&self, command: &[&str]) -> Result<String> {
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .with_context(|| format!("Falha ao conectar no socket mpv em {:?}", self.socket_path))?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        let cmd_json = serde_json::json!({
            "command": command
        });
        let mut payload = cmd_json.to_string();
        payload.push('\n');

        writer.write_all(payload.as_bytes()).await?;

        let mut line = String::new();
        reader.read_line(&mut line).await?;
        debug!("Resposta mpv IPC: {}", line.trim());
        Ok(line)
    }

    pub async fn pause(&self) -> Result<()> {
        self.send_command(&["set_property", "pause", "yes"]).await?;
        Ok(())
    }

    pub async fn resume(&self) -> Result<()> {
        self.send_command(&["set_property", "pause", "no"]).await?;
        Ok(())
    }

    pub async fn set_volume(&self, vol: u32) -> Result<()> {
        let vol_str = vol.to_string();
        self.send_command(&["set_property", "volume", &vol_str]).await?;
        Ok(())
    }
}

impl Drop for MpvManager {
    fn drop(&mut self) {
        self.stop();
    }
}
