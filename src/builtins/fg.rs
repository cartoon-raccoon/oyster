use nix::unistd::{getpgid, Pid};
use nix::sys::signal::{Signal, killpg};

use crate::types::Cmd;
use crate::shell::{self, Shell};
use crate::jobc;

pub fn run(sh: &mut Shell, cmd: Cmd) -> i32 {
    if cmd.args.len() > 2 {
        eprintln!("fg: too many arguments");
        return 2;
    }
    let mut job_id = 0;
    let mut pgid = Pid::from_raw(0);
    let mut pids = Vec::new();
    let mut command = String::new();
    if cmd.args.len() == 1 {
        if sh.jobs.len() > 1 {
            eprintln!("fg: more than one job running");
            return 2;
        }
        //this loop should only run once
        for (id, job) in sh.jobs.iter() {
            job_id = *id;
            pgid = job.pgid;
            pids = job.pids.clone();
            command = job.firstcmd.clone();
        }
    } else if cmd.args.len() == 2 {
        match cmd.args[1].parse::<i32>() {
            Ok(id) => {
                job_id = id;
                if let Some(job) = sh.get_job_by_id(id) {
                    pgid = job.pgid;
                    pids = job.pids.clone();
                    command = job.firstcmd.clone();
                } else {
                    eprintln!("fg: no such job");
                    return 2;
                }
            }
            Err(_) => {
                eprintln!("fg: please enter a numeric id");
                return 3;
            }
        }
    }
    if pgid == Pid::from_raw(0) || command.is_empty(){ 
        eprintln!("fg: job not found");
        return 2;
    }
    eprintln!("[{}] {}", job_id, command);
    match shell::give_terminal_to(pgid) {
        Ok(_) => {
            match killpg(pgid, Signal::SIGCONT) {
                Ok(()) => {
                    jobc::mark_job_as_running(sh, job_id, false);
                }
                Err(e) => {
                    eprintln!("fg: error sending SIGCONT: {}", e);
                    return 4;
                }
            }
        }
        Err(_) => {
            eprintln!("fg: error giving terminal");
            return 4;
        }
    }
    let mut status = 0;
    for pid in pids.iter() {
        status = jobc::wait_on_job(sh, pgid, *pid, true);
    }

    //the unwrap shouldn't fail because the shell is always in a pg
    let shell_pgid = getpgid(Some(Pid::from_raw(0))).unwrap();
    match shell::give_terminal_to(shell_pgid) {
        Ok(_) => {
            return status;
        }
        Err(_) => {
            eprintln!("fg: error giving terminal");
            return 4;
        }
    }
}