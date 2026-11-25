use crate::custom_event::{RuffleEvent, RuffleEventSender};

use ruffle_core::external::FsCommandProvider;

pub struct DesktopFSCommandProvider {
    pub event_loop: RuffleEventSender,
}

impl FsCommandProvider for DesktopFSCommandProvider {
    fn on_fs_command(&self, command: &str, args: &str) -> bool {
        match command {
            "quit" => {
                let _ = self.event_loop.send_event(RuffleEvent::ExitRequested);
            }
            "fullscreen" => {
                match args {
                    "true" => {
                        let _ = self.event_loop.send_event(RuffleEvent::EnterFullScreen);
                    }
                    "false" => {
                        let _ = self.event_loop.send_event(RuffleEvent::ExitFullScreen);
                    }
                    _ => {}
                };
            }
            _ => return false,
        };

        true
    }
}
