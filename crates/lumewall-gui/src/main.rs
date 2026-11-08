use chrono::Utc;
use gtk4::gio;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use lumewall_backend::{DesktopBackend, GnomeBackend};
use lumewall_core::{calculate_solar_position, Config, Profile};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
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
    gtk4::Window::set_default_icon_name(APP_ID);

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
    let backend = Rc::new(GnomeBackend);

    let window = libadwaita::ApplicationWindow::builder()
        .application(app)
        .title("LumeWall")
        .icon_name(APP_ID)
        .default_width(820)
        .default_height(640)
        .build();

    let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

    let header = libadwaita::HeaderBar::new();
    content_box.append(&header);

    let scrolled = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .build();

    let clamp = libadwaita::Clamp::builder()
        .maximum_size(760)
        .margin_top(16)
        .margin_bottom(24)
        .margin_start(16)
        .margin_end(16)
        .build();

    let page_box = gtk4::Box::new(gtk4::Orientation::Vertical, 18);

    // 1. Grupo Wallpaper Ativo & Status Solar
    let status_group = libadwaita::PreferencesGroup::builder()
        .title("Papel de Parede Ativo")
        .description("Controle de alternância e estado atual do desktop")
        .build();

    let current_wall = backend
        .get_current_wallpaper()
        .unwrap_or_else(|_| "Desconhecido".to_string())
        .replace("file://", "");

    let active_wall_row = libadwaita::ActionRow::builder()
        .title("Papel de Parede Atual")
        .subtitle(&current_wall)
        .build();
    active_wall_row.add_prefix(&gtk4::Image::from_icon_name("preferences-desktop-wallpaper-symbolic"));

    let coords = config_state.borrow().location;
    let pos = calculate_solar_position(&coords, Utc::now());
    let solar_info = format!("Elevação: {:.1}° | Azimute: {:.1}°", pos.altitude, pos.azimuth);

    let solar_row = libadwaita::ActionRow::builder()
        .title("Sol em Tempo Real (NOAA)")
        .subtitle(&solar_info)
        .build();
    solar_row.add_prefix(&gtk4::Image::from_icon_name("weather-clear-symbolic"));

    status_group.add(&active_wall_row);
    status_group.add(&solar_row);
    page_box.append(&status_group);

    // 2. Grupo: Escolher Wallpaper Avulso e Ativar Imediatamente
    let choose_group = libadwaita::PreferencesGroup::builder()
        .title("Escolher e Ativar Wallpaper")
        .description("Selecione qualquer imagem do seu computador para definir agora como papel de parede")
        .build();

    let choose_row = libadwaita::ActionRow::builder()
        .title("Selecionar Imagem do Computador")
        .subtitle("Abre o seletor de arquivos (JPG, PNG, WebP)")
        .build();
    choose_row.add_prefix(&gtk4::Image::from_icon_name("document-open-symbolic"));

    let select_btn = gtk4::Button::builder()
        .label("Escolher Arquivo...")
        .valign(gtk4::Align::Center)
        .css_classes(["suggested-action"])
        .build();

    {
        let window_clone = window.clone();
        let backend_clone = backend.clone();
        let active_wall_clone = active_wall_row.clone();

        select_btn.connect_clicked(move |_| {
            let filter = gtk4::FileFilter::new();
            filter.set_name(Some("Imagens (*.jpg, *.png, *.webp)"));
            filter.add_mime_type("image/jpeg");
            filter.add_mime_type("image/png");
            filter.add_mime_type("image/webp");

            let dialog = gtk4::FileDialog::builder()
                .title("Selecionar Papel de Parede")
                .modal(true)
                .build();

            let filters = gio::ListStore::new::<gtk4::FileFilter>();
            filters.append(&filter);
            dialog.set_filters(Some(&filters));

            let backend_inner = backend_clone.clone();
            let active_wall_inner = active_wall_clone.clone();

            dialog.open(Some(&window_clone), gio::Cancellable::NONE, move |result| {
                if let Ok(file) = result {
                    if let Some(path) = file.path() {
                        if let Err(e) = backend_inner.set_wallpaper(&path, None) {
                            eprintln!("Erro ao definir wallpaper: {}", e);
                        } else {
                            active_wall_inner.set_subtitle(&path.display().to_string());
                        }
                    }
                }
            });
        });
    }

    choose_row.add_suffix(&select_btn);
    choose_group.add(&choose_row);
    page_box.append(&choose_group);

    // 3. Galeria Local: Wallpapers na pasta ~/Pictures/LumeWall/Wallpapers
    let gallery_group = libadwaita::PreferencesGroup::builder()
        .title("Galeria Local de Wallpapers")
        .description("Wallpapers encontrados em ~/Pictures/LumeWall/Wallpapers")
        .build();

    let wallpapers_dir = config_state.borrow().storage.wallpapers_dir.clone();
    if let Ok(entries) = fs::read_dir(&wallpapers_dir) {
        let mut image_files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                if let Some(ext) = p.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    matches!(ext_str.as_str(), "jpg" | "jpeg" | "png" | "webp")
                } else {
                    false
                }
            })
            .collect();

        image_files.sort();

        for img_path in image_files {
            let file_name = img_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let row = libadwaita::ActionRow::builder()
                .title(&file_name)
                .subtitle(&img_path.display().to_string())
                .build();

            row.add_prefix(&gtk4::Image::from_icon_name("image-x-generic-symbolic"));

            let activate_btn = gtk4::Button::builder()
                .label("Ativar")
                .valign(gtk4::Align::Center)
                .build();

            let path_clone = img_path.clone();
            let backend_clone = backend.clone();
            let active_wall_clone = active_wall_row.clone();

            activate_btn.connect_clicked(move |_| {
                if let Err(e) = backend_clone.set_wallpaper(&path_clone, None) {
                    eprintln!("Erro ao ativar wallpaper: {}", e);
                } else {
                    active_wall_clone.set_subtitle(&path_clone.display().to_string());
                }
            });

            row.add_suffix(&activate_btn);
            gallery_group.add(&row);
        }
    }

    page_box.append(&gallery_group);

    // 4. Grupo: Perfis Dinâmicos (Solar, Horário, Vídeo)
    let profiles_group = libadwaita::PreferencesGroup::builder()
        .title("Perfis Dinâmicos")
        .description("Perfis com ciclos automáticos solares e temporais")
        .build();

    for (name, prof) in &config_state.borrow().profiles {
        let profile_row = libadwaita::ActionRow::builder().title(name).build();

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
            .label("Ativar Perfil")
            .valign(gtk4::Align::Center)
            .css_classes(["suggested-action"])
            .build();

        let profile_name = name.clone();
        let config_clone = config_state.clone();
        let config_p = config_path.clone();
        let active_wall_clone = active_wall_row.clone();

        apply_btn.connect_clicked(move |_| {
            let mut cfg = config_clone.borrow_mut();
            cfg.general.active_profile = profile_name.clone();
            let _ = cfg.save_to_file(&config_p);
            active_wall_clone.set_subtitle(&format!("Perfil: {}", profile_name));

            // Avisa o lumewalld
            let _ = std::process::Command::new("pkill")
                .args(["-HUP", "lumewalld"])
                .status();
        });

        profile_row.add_suffix(&apply_btn);
        profiles_group.add(&profile_row);
    }

    page_box.append(&profiles_group);

    // 5. Pastas Fixas
    let storage_group = libadwaita::PreferencesGroup::builder()
        .title("Gerenciar Pastas de Mídia")
        .build();

    let wall_dir_row = libadwaita::ActionRow::builder()
        .title("Pasta de Wallpapers")
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

    storage_group.add(&wall_dir_row);
    page_box.append(&storage_group);

    clamp.set_child(Some(&page_box));
    scrolled.set_child(Some(&clamp));
    content_box.append(&scrolled);

    window.set_content(Some(&content_box));
    window.present();
}
