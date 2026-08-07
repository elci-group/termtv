//! Platform-specific process lifetime and signaling helpers.

use std::process::Command;

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
