//! Platform-specific process lifetime and signaling helpers.

use std::{io, path::PathBuf, process::Command};

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

/// Send a signal to a child process by PID.
pub(crate) fn signal_child(pid: u32, signal: &str) {
    match Command::new("kill")
        .args([format!("-{signal}"), pid.to_string()])
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!(
            "termtv_event=controls operation=signal_child signal={signal} pid={pid} status={status}"
        ),
        Err(error) => eprintln!(
            "termtv_event=controls operation=signal_child signal={signal} pid={pid} error={error}"
        ),
    }
}

/// Ensure child FFmpeg/ffplay processes terminate if the parent dies.
#[cfg(target_os = "linux")]
pub(crate) fn configure_child_lifetime(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            unsafe extern "C" {
                fn prctl(option: i32, arg2: i32, arg3: usize, arg4: usize, arg5: usize) -> i32;
            }
            if prctl(1, 15, 0, 0, 0) != 0 {
                eprintln!(
                    "termtv_event=process operation=set_parent_death_signal error=prctl_failed"
                );
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn configure_child_lifetime(_command: &mut Command) {}

/// Return the PID-specific camera stop file path.
pub(crate) fn camera_stop_path(pid: u32) -> PathBuf {
    PathBuf::from(format!("/tmp/termtv-camera.{pid}.stop"))
}

/// Check whether a stop signal exists for the given camera process.
pub(crate) fn camera_stop_requested(pid: u32) -> bool {
    camera_stop_path(pid).is_file()
}

/// Remove the stop signal for the given camera process, ignoring missing file.
pub(crate) fn clear_camera_stop(pid: u32) {
    let _ = std::fs::remove_file(camera_stop_path(pid));
}

/// Request stop for all running termtv camera sessions.
pub fn request_camera_stop_all() -> io::Result<()> {
    for entry in std::fs::read_dir("/tmp")? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("termtv-camera.") && name.ends_with(".stop") {
            std::fs::write(entry.path(), "stop")?;
        }
    }
    Ok(())
}

/// List candidate V4L2 camera devices.
#[cfg(target_os = "linux")]
pub fn available_camera_devices() -> io::Result<Vec<PathBuf>> {
    let mut devices = Vec::new();
    for entry in std::fs::read_dir("/dev")? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("video") {
            devices.push(entry.path());
        }
    }
    devices.sort();
    Ok(devices)
}

#[cfg(not(target_os = "linux"))]
pub fn available_camera_devices() -> io::Result<Vec<PathBuf>> {
    Ok(Vec::new())
}
