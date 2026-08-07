//! Non-blocking terminal input controls.

use std::{
    io::{self, Read},
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
};

/// Capture raw keyboard input without blocking the render loop.
pub(crate) struct TerminalControls {
    pub(crate) receiver: Receiver<u8>,
    restore: Option<String>,
}

impl TerminalControls {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let restore = match Command::new("stty").arg("-g").output() {
            Ok(output) if output.status.success() => match String::from_utf8(output.stdout) {
                Ok(state) => Some(state),
                Err(_) => {
                    eprintln!(
                        "termtv_event=controls operation=decode_tty_state error=invalid_utf8"
                    );
                    None
                }
            },
            Ok(_) => None,
            Err(error) => {
                eprintln!("termtv_event=controls operation=probe_tty error={error}");
                None
            }
        };
        if restore.is_some() {
            match Command::new("stty")
                .args(["-icanon", "-echo", "min", "0", "time", "1"])
                .status()
            {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    eprintln!("termtv_event=controls operation=enable_raw status={status}")
                }
                Err(error) => {
                    eprintln!("termtv_event=controls operation=enable_raw error={error}")
                }
            }
            let _reader = thread::spawn(move || {
                let mut input = io::stdin();
                loop {
                    let mut byte = [0; 1];
                    match input.read(&mut byte) {
                        Ok(1) if sender.send(byte[0]).is_err() => break,
                        Ok(1) => {}
                        Ok(0) => break,
                        Ok(_) => break,
                        Err(error) => {
                            eprintln!("termtv_event=controls operation=read_input error={error}");
                            break;
                        }
                    }
                }
            });
        }
        Self { receiver, restore }
    }
}

impl Drop for TerminalControls {
    fn drop(&mut self) {
        if let Some(state) = self.restore.take() {
            match Command::new("stty").arg(state.trim()).status() {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    eprintln!("termtv_event=controls operation=restore_tty status={status}")
                }
                Err(error) => {
                    eprintln!("termtv_event=controls operation=restore_tty error={error}")
                }
            }
        }
    }
}
