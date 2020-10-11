use std::os::unix::process::CommandExt;
use std::process::Command;

use nix::sys::signal::{signal, Signal, SigHandler,};

use crate::parser::Cmd;

#[must_use]
pub fn execute(cmd: Cmd) -> bool {
    let mut command = Command::new(cmd.cmd);
    if let Ok(mut child) = unsafe { 
        command
        .args(cmd.args)
        .pre_exec(|| {
                match signal(Signal::SIGINT, SigHandler::SigDfl) {
                    Ok(_) => {},
                    Err(_) => {panic!("Could not set SIGINT handler")}
                };
                match signal(Signal::SIGQUIT, SigHandler::SigDfl) {
                    Ok(_) => {},
                    Err(_) => {panic!("Could not set SIGQUIT handler")}
                }
            Ok(())
        }).spawn() } {
            child.wait().expect("Child wasn't running").success()
    } else {
        eprintln!("Command not found!");
        true
    }
}