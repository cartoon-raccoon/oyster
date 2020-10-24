use nix::unistd::Pid;
use nix::sys::signal::{Signal, killpg};

use crate::types::Cmd;
use crate::shell::Shell;
use crate::jobc;

pub fn run(shell: &mut Shell, cmd: Cmd) -> i32 {
    if cmd.args.len() > 2 {
        eprintln!("bg: too many arguments");
        return 1;
    }
    let mut job_id = 0;
    let mut pgid = Pid::from_raw(0);
    if cmd.args.len() == 1 {
        if shell.jobs.len() > 1 {
            eprintln!("bg: more than one job running");
            return 2;
        }
        //this loop should only run once
        for (id, job) in shell.jobs.iter() {
            job_id = *id;
            pgid = job.pgid;
        }
    } else if cmd.args.len() == 2 {
        match cmd.args[1].parse::<i32>() {
            Ok(id) => {
                job_id = id;
                if let Some(job) = shell.get_job_by_id(id) {
                    pgid = job.pgid;
                } else {
                    eprintln!("bg: no such job");
                    return 2;
                }
            }
            Err(_) => {
                eprintln!("bg: please enter a numeric id");
                return 3;
            }
        }
    }
    if pgid == Pid::from_raw(0) {
        eprintln!("bg: job not found");
        return 2;
    }
    match killpg(pgid, Signal::SIGCONT) {
        Ok(()) => {
            jobc::mark_job_as_running(shell, job_id, true);
        }
        Err(e) => {
            eprintln!("bg: error sending SIGCONT: {}", e);
            return 4;
        }
    }
    0
}