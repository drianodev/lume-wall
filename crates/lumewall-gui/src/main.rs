use chrono::Utc;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use lumewall_core::{calculate_solar_position, Config, Profile};
use std::cell::RefCell;
use std::rc::Rc;

const APP_ID: &str = "io.github.drianodev.LumeWall";

fn main() {
    let app = libadwaita::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &libadwaita::Application) {
    let config_path = Config::default_config_path().unwrap();
    let config = if config_path.exists() {
        Config::load_from_file(&config_path).unwrap_or_else(|_| Config::create_default_scaffold())
    } else {
        let sc = Config::create_default_scaffold();
        let _ = sc.save_to_file(&config_path);
        sc
    };
    let _ = config.ensure_storage_dirs();

    let config_state = Rc::new(RefCell::new(config));

    // Janela Principal Libadwaita
    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("LumeWall")
        .default_width(780)
        .default_height(580)
        .build();

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    // HeaderBar
    let header = libadwaita::HeaderBar::new();
    content_box.append(&header);

    // Scrolled window para conteúdo
    let scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .build();

    let clamp = libadwaita::Clamp::builder()
        .maximum_size(720)
        .margin_top(16)
        .margin_bottom(24)
        .margin_start(16)
        .margin_end(16)
        .build();

    let page_box = gtk4::Box::new(gtk4::Orientation::Vertical, 16);

    // 1. Grupo Status e Wallpaper Ativo
    let status_group = libadwaita::PreferencesGroup::builder()
        .title("Status do Sistema")
        .description("Controle de alternância dinâmica de papéis de parede")
        .build();

    let active_profile_row = libadwaita::ActionRow::builder()
        .title("Perfil Atual")
        .subtitle(&config_state.borrow().general.active_profile)
        .build();

    // Dados Solares
    let coords = config_state.borrow().location;
    let pos = calculate_solar_position(&coords, Utc::now());
    let solar_info = format!("Elevação: {:.1}° | Azimute: {:.1}°", pos.altitude, pos.azimuth);

    let solar_row = libadwaita::ActionRow::builder()
        .title("Sol em Tempo Real (NOAA)")
        .subtitle(&solar_info)
        .build();
    solar_row.add_prefix(&gtk4::Image::from_icon_name("weather-clear-symbolic"));

    status_group.add(&active_profile_row);
    status_group.add(&solar_row);
    page_box.append(&status_group);

    // 2. Grupo Pastas Fixas de Mídia
    let storage_group = libadwaita::PreferencesGroup::builder()
        .title("Diretórios de Mídia Fixos")
        .description("Seus papéis de parede e vídeos são organizados nestas pastas")
        .build();

    let wall_dir_row = libadwaita::ActionRow::builder()
        .title("Pasta de Wallpapers (Imagens)")
        .subtitle(&config_state.borrow().storage.wallpapers_dir.display().to_string())
        .build();

    let open_wall_btn = gtk4::Button::builder()
        .label("Abrir Pasta")
        .valign(gtk4::Align::Center)
        .build();

    let wall_dir = config_state.borrow().storage.wallpapers_dir.clone();
    open_wall_btn.connect_clicked(move |_| {
        let _ = std::process::Command::new("xdg-open").arg(&wall_dir).spawn();
    });
    wall_dir_row.add_suffix(&open_wall_btn);

    let video_dir_row = libadwaita::ActionRow::builder()
        .title("Pasta de Vídeos")
        .subtitle(&config_state.borrow().storage.videos_dir.display().to_string())
        .build();

    let open_video_btn = gtk4::Button::builder()
        .label("Abrir Pasta")
        .valign(gtk4::Align::Center)
        .build();

    let video_dir = config_state.borrow().storage.videos_dir.clone();
    open_video_btn.connect_clicked(move |_| {
        let _ = std::process::Command::new("xdg-open").arg(&video_dir).spawn();
    });
    video_dir_row.add_suffix(&open_video_btn);

    storage_group.add(&wall_dir_row);
    storage_group.add(&video_dir_row);
    page_box.append(&storage_group);

    // 3. Grupo de Seleção e Ativação de Perfis
    let profiles_group = libadwaita::PreferencesGroup::builder()
        .title("Perfis de Wallpaper")
        .description("Selecione um perfil para ativar imediatamente")
        .build();

    for (name, prof) in &config_state.borrow().profiles {
        let profile_row = libadwaita::ActionRow::builder()
            .title(name)
            .build();

        let icon_name = match prof {
            Profile::Solar { description, frames } => {
                let desc = description.clone().unwrap_or_else(|| format!("{} frames solares", frames.len()));
                profile_row.set_subtitle(&format!("Solar • {}", desc));
                "weather-clear-symbolic"
            }
            Profile::Time { description, frames } => {
                let desc = description.clone().unwrap_or_else(|| format!("{} frames de horário", frames.len()));
                profile_row.set_subtitle(&format!("Horário • {}", desc));
                "preferences-system-time-symbolic"
            }
            Profile::Video { description, .. } => {
                let desc = description.clone().unwrap_or_else(|| "Wallpaper de vídeo".to_string());
                profile_row.set_subtitle(&format!("Vídeo • {}", desc));
                "video-x-generic-symbolic"
            }
        };

        profile_row.add_prefix(&gtk4::Image::from_icon_name(icon_name));

        let apply_btn = gtk4::Button::builder()
            .label("Ativar")
            .valign(gtk4::Align::Center)
            .css_classes(["suggested-action"])
            .build();

        let profile_name = name.clone();
        let config_clone = config_state.clone();
        let config_p = config_path.clone();
        let active_row_clone = active_profile_row.clone();

        apply_btn.connect_clicked(move |_| {
            let mut cfg = config_clone.borrow_mut();
            cfg.general.active_profile = profile_name.clone();
            let _ = cfg.save_to_file(&config_p);
            active_row_clone.set_subtitle(&profile_name);

            // Notifica o daemon via SIGHUP
            let _ = std::process::Command::new("pkill")
                .args(["-HUP", "lumewalld"])
                .status();
        });

        profile_row.add_suffix(&apply_btn);
        profiles_group.add(&profile_row);
    }

    page_box.append(&profiles_group);

    clamp.set_child(Some(&page_box));
    scrolled.set_child(Some(&clamp));
    content_box.append(&scrolled);

    window.set_content(Some(&content_box));
    window.present();
}
