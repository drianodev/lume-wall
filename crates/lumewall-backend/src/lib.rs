pub mod desktop;
pub mod mpv_ipc;

pub use desktop::{DesktopBackend, GnomeBackend};
pub use mpv_ipc::MpvManager;
