use nix::unistd::{Pid, write};
use nix::sys::{
    wait::{waitpid, WaitPidFlag, WaitStatus},
    signal::Signal,
};
use nix::Error;
use nix::errno::Errno;

use crate::types::{
    JobStatus,
    JobTrack,
    STOPPED,
};
use crate::shell::Shell;

pub fn print_job(job: &JobTrack) {
    let to_print = format!("\n[{}] {} {} {}\n",
    job.id, job.pgid, job.firstcmd, job.status);
    if job.background {
        write(1, to_print.as_bytes()).unwrap();
    } else if job.status == JobStatus::Signaled(Signal::SIGSEGV) {
        write(1, to_print.as_bytes()).unwrap();
    } else if job.status == JobStatus::Stopped {
        write(1, to_print.as_bytes()).unwrap();
    }
}

pub fn mark_job_as_stopped(shell: &mut Shell, id: i32) {
    shell.mark_job_as_stopped(id);
    if let Some(job) = shell.get_job_by_id(id) {
        print_job(job);
    }
}

pub fn mark_job_as_running(shell: &mut Shell, id: i32, bg: bool) {
    shell.mark_job_as_running(id, bg);
    if let Some(job) = shell.get_job_by_id(id) {
        print_job(job);
    }
}

pub fn cleanup_process(
    shell: &mut Shell, 
    pid: Pid, 
    pgid: Pid,
    status: JobStatus) {
    if let Some(mut job) = shell.remove_pid_from_job(pid, pgid) {
        job.status = status;
        print_job(&job);
    }
}

pub fn try_wait_bg_jobs(shell: &mut Shell) {
    if shell.jobs.is_empty() {
        return;
    }
    let jobs = shell.jobs.clone();
    for (_i, job) in jobs.iter() {
        for pid in job.pids.iter() {
            wait_on_job(shell, job.pgid, *pid, false);
        }
    }
}

pub fn wait_on_job(
    shell: &mut Shell, 
    pgid: Pid, 
    pid: Pid, 
    stop: bool
) -> i32 {
    let mut status = 0;
    let flags: Option<WaitPidFlag>;
    if stop {
        flags = Some(WaitPidFlag::WUNTRACED);
    } else {
        flags = Some(WaitPidFlag::WNOHANG);
    }
    match waitpid(pid, flags) {
        Ok(result) => {
            match result {
                WaitStatus::Exited(pid, exitstat) => {
                    cleanup_process(shell, pid, pgid, JobStatus::Completed(exitstat));
                    status = exitstat;
                }
                WaitStatus::Stopped(_pid, _signal) => {
                    status = STOPPED;
                }
                WaitStatus::Signaled(pid, signal, _cd) => {
                    cleanup_process(shell, pid, pgid, JobStatus::Signaled(signal));
                    status = signal as i32;
                }
                _ => {

                }
            }
        }
        Err(error) =>{
            match error {
                Error::Sys(errno) => {
                    if errno == Errno::ECHILD {
                        cleanup_process(shell, pid, pgid, JobStatus::Completed(status));
                    }
                }
                _ => {
                    //handle error
                }
            }
        }
    }
    status
}