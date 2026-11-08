# 🌅 LumeWall

> **Dynamic & Video Wallpaper Manager for Ubuntu / Linux**  
> Um gerenciador unificado e de consumo ultrabaixo de papéis de parede dinâmicos (ciclos solares e horários) e vídeos em loop (`mpv`) para Ubuntu / GNOME.

---

## ✨ Funcionalidades

- ☀️ **Cálculo Solar Offline (NOAA SPA):** Ajusta os frames de papel de parede automaticamente pelo ângulo e azimute do sol em tempo real sem chamadas de rede.
- 🕒 **Horário Fixo:** Agende trocas por horários específicos do dia (ex: 06:00, 12:00, 18:00).
- 🎥 **Wallpapers em Vídeo (`mpv`):** Roda vídeos em loop com decodificação de hardware nativa (VA-API/NVDEC) via socket IPC assíncrono.
- 🚀 **Consumo Mínimo de Recursos:** Daemon em background consumindo menos de 1 MB de memória RAM.
- 🎨 **Interface Nativa GTK4 / Libadwaita:** Segue o GNOME HIG com alternância instantânea de temas.
- 📁 **Diretórios de Mídia Fixos:** Armazenamento organizado em `~/Pictures/LumeWall/Wallpapers` e `~/Videos/LumeWall`.
- ⚙️ **Hot-Reload:** Atualiza perfis via `config.toml` sem reiniciar o daemon (`SIGHUP`).

---

## 🛠️ Instalação & Compilação

### Dependências no Ubuntu 24.04:
```bash
sudo apt update && sudo apt install -y build-essential pkg-config libgtk-4-dev libadwaita-1-dev mpv
```

### Compilando e Instalando:
```bash
# Compilar versão de release otimizada
cargo build --release

# Instalar binários
cp target/release/lumewalld ~/.local/bin/
cp target/release/lumewall-cli ~/.local/bin/lumewall
cp target/release/lumewall-gui ~/.local/bin/

# Instalar serviço systemd em segundo plano
mkdir -p ~/.config/systemd/user
cp data/lumewall.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now lumewall.service

# Instalar ícone e atalho no menu
mkdir -p ~/.local/share/icons/hicolor/scalable/apps ~/.local/share/applications
cp data/icons/io.github.drianodev.LumeWall.svg ~/.local/share/icons/hicolor/scalable/apps/
cp data/io.github.drianodev.LumeWall.desktop ~/.local/share/applications/
update-desktop-database ~/.local/share/applications/
```

---

## 🖥️ Uso

### Interface Gráfica
Abra pelo menu de aplicativos procurando por **LumeWall** ou execute:
```bash
lumewall-gui
```

### Linha de Comando (CLI)
```bash
# Verificar status atual
lumewall status

# Verificar dados solares astronômicos em tempo real
lumewall solar

# Listar perfis configurados
lumewall list

# Ativar um perfil
lumewall set-profile solar-local
```

---

## 📄 Licença
Distribuído sob licença MIT. Criado por Adriano (drianodev).
