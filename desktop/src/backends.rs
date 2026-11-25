mod external_interface;
mod fscommand;
#[cfg(not(target_os = "wasi"))]
mod navigator;
mod path_allow_list;
mod ui;
#[cfg(target_os = "wasi")]
mod waos;

pub use external_interface::DesktopExternalInterfaceProvider;
pub use fscommand::DesktopFSCommandProvider;
#[cfg(not(target_os = "wasi"))]
pub use navigator::DesktopNavigatorInterface;
pub use path_allow_list::PathAllowList;
pub use ui::DesktopUiBackend;
pub use ui::DeviceFontRenderer;
#[cfg(target_os = "wasi")]
pub use waos::WaosNavigatorBackend;

use ruffle_core::backend::audio::AudioBackend;

// WaOS port: the player's sound, by the system that plays it.

/// The player's sound, on the output named, the system's own where none is: cpal's on a desktop.
#[cfg(not(target_os = "wasi"))]
pub fn audio(output_device: Option<&str>) -> Result<impl AudioBackend, String> {
    ruffle_frontend_utils::backends::audio::CpalAudioBackend::new(output_device)
        .map_err(|error| error.to_string())
}

/// The player's sound, through WaOS's audio graph, on the output WaOS's user chose for the app:
/// a name the preferences give is not read.
#[cfg(target_os = "wasi")]
pub fn audio(_output_device: Option<&str>) -> Result<impl AudioBackend, String> {
    waos::WaosAudioBackend::start()
}

/// The sound outputs the preferences offer by name; none on WaOS, whose user chooses an app's
/// output in WaOS's own settings.
pub fn output_device_names() -> Vec<String> {
    #[cfg(not(target_os = "wasi"))]
    {
        use cpal::traits::{DeviceTrait, HostTrait};

        let Ok(devices) = cpal::default_host().output_devices() else {
            return Vec::new();
        };
        devices.filter_map(|device| device.name().ok()).collect()
    }
    #[cfg(target_os = "wasi")]
    Vec::new()
}
