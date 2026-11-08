use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, info};

pub struct MpvIpcClient {
    socket_path: PathBuf,
}

impl MpvIpcClient {
    pub fn new<P: AsRef<Path>>(socket_path: P) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
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

    pub async fn load_file(&self, path: &Path) -> Result<()> {
        let p_str = path.to_string_lossy();
        info!("Enviando loadfile para mpv: {}", p_str);
        self.send_command(&["loadfile", &p_str]).await?;
        Ok(())
    }

    pub async fn pause(&self) -> Result<()> {
        self.send_command(&["set_property", "pause", "yes"]).await?;
        Ok(())
    }

    pub async fn resume(&self) -> Result<()> {
        self.send_command(&["set_property", "pause", "no"]).await?;
        Ok(())
    }
}
