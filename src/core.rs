use std::fs::File;
use std::io::Read;
use std::os::unix::io::{
    RawFd,
    FromRawFd,
};
use std::process;
use std::ffi::{CString, CStr};

use nix::unistd::{
    Pid,
    getpid,
    setpgid,
    isatty,
    fork, 
    pipe, 
    execvp, 
    dup, dup2, 
    close,
    ForkResult
};
use nix::Result;
use nix::Error;
use nix::errno::Errno;

use crate::shell;

//placeholder - move to job control
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};

use crate::types::*;
use crate::shell::Shell;

/// Even lower level, it deconstructs the job
/// and passes raw parameters to the final function.
/// Also handles command expansion logic.
pub fn run_pipeline(
    shell: &mut Shell, 
    job: Job, 
    background: bool, 
    capture: bool) -> Result<(bool, CommandResult)> {
    
    //defaults to return
    let mut term_given = false;
    let mut cmdresult = CommandResult::new();

    //making vec of pipes
    let mut pipes = Vec::new();
    for _ in 0..job.cmds.len() - 1 {
        match pipe() {
            Ok(fdpair) => {
                pipes.push(fdpair);
            }
            Err(e) => { return Err(e); }
        }
    }

    let isatty = isatty(1)?;

    let mut pgid = Pid::from_raw(0);
    let mut idx: usize = 0;
    let mut children = Vec::new();

    for cmd in job.cmds {

        let params = CommandParams{
            isatty: isatty,
            background: background,
            capture_output: capture, //is used in command expansion only
            env: shell.env.clone(),
        };

        let childpid = run_command(
            cmd, 
            idx, 
            &mut pgid,
            &pipes,
            params,
            &mut term_given,
            &mut cmdresult,
        )?;
        if childpid > 0 && !background {
            children.push(childpid);
        }
        idx += 1;
    }

    //placeholder code - waiting will be moved to job control
    for childpid in children {
        match waitpid(Pid::from_raw(childpid), Some(WaitPidFlag::WUNTRACED))? {
            WaitStatus::Exited(_pid, status) => {
                cmdresult.status = status;
            }
            WaitStatus::Signaled(pid, signal, cd) => {
                println!("process {} was signaled by {}: coredump? {}", pid, signal, cd);
            }
            WaitStatus::Stopped(pid, signal) => {
                println!("process {} was stopped by {}", pid, signal);
            }
            status @ _ => {
                println!("process {} has status {:?}", childpid, status);
            }
        }
    }

    for pipe in pipes {
        close(pipe.0)?;
        close(pipe.1)?;
    }

    Ok((term_given, cmdresult))
}

/// This is one deep-ass core function.
fn run_command(
    cmd: Cmd, 
    idx: usize, 
    pgid: &mut Pid,
    pipes: &Vec<(RawFd, RawFd)>,
    params: CommandParams,
    term_given: &mut bool,
    // for capturing output
    results: &mut CommandResult,) -> Result<i32> {

    let fds_capture_stdout = pipe()?;
    let fds_capture_stderr = pipe()?;

    const FD_DUPLICATE_ERR: &'static str =
        "oyster: failed to duplicate file descriptor";
    const PIPE_CONNECT_ERR: &'static str = 
        "oyster: failed to connect pipes";
    const PIPE_END_CLOSE_ERR: &'static str =
        "oyster: could not close pipe file descriptor";
    const PGID_SET_ERR: &'static str = 
        "oyster: failed to set pgid for child";

    let pipes_count = pipes.len();
    let forkresult = fork()?;

    match forkresult {
        //cannot return errors from here, have to exit
        ForkResult::Child => {
            //setting process groups
            let pid = getpid();
            if idx == 0 {
                *pgid = pid; //setting pgid to own pid
                setpgid(Pid::from_raw(0), pid)
                    .unwrap_or_exit(PGID_SET_ERR, 2);
            } else {
                setpgid(Pid::from_raw(0), *pgid)
                    .unwrap_or_exit(PGID_SET_ERR, 2);
            }

            //connecting up pipes for commands to read from
            if idx > 0 { //not the first command
                let fds = pipes[idx - 1];
                dup2(fds.0, 0).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                close(fds.0).unwrap_or_exit(PIPE_CONNECT_ERR, 4);
            }

            //connecting up pipes for commands to write to
            if idx < pipes_count {
                let fds = pipes[idx];
                dup2(fds.1, 1).unwrap_or_exit(PIPE_CONNECT_ERR, 4);
            }

            let mut stdout_redirected = false;
            //let mut stderr_redirected = false;

            for redirect in &cmd.redirects {
                
                if redirect.0 == "&2" && redirect.2 == "&1" {
                    if idx < pipes_count {
                        let fds = pipes[idx];
                        dup2(fds.1, 2).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                    } else if !params.capture_output {
                        let fd = dup(2).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                        dup2(fd, 2).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                    }
                    //ya wanna redirect wih output capture enabled?
                    //sure, don't blame me for any shitfuckery that happens.
                } else if redirect.0 == "1" && redirect.2 == "&2" {
                    if idx < pipes_count || !params.capture_output {
                        let fd = dup(2).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                        dup2(fd, 1).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                    }
                } else if redirect.0 == "1" && redirect.2 == "&1" ||
                          redirect.0 == "2" && redirect.2 == "&2" {
                    //do nothing because no one would do this
                    //but we need to emulate zsh's behaviour
                    //if we didn't catch this case, 
                    //1>&1 would redirect stdin to a file called "&1"
                } else {
                    let to_append = redirect.1 == Redirect::Append;
                    let fd = shell::create_fd_from_file(&redirect.2, to_append);
                    if redirect.0 == "1" {
                        dup2(fd, 1).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                        stdout_redirected = true;
                    } else {
                        dup2(fd, 2).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                        //stderr_redirected = true;
                    }
                }
            }

            //TODO: Fix this: stdout not redirecting properly
            if idx == pipes_count && params.capture_output {
                if !stdout_redirected {
                    close(fds_capture_stdout.0).unwrap_or_exit(PIPE_END_CLOSE_ERR, 4);
                    dup2(fds_capture_stdout.1, 1).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                    close(fds_capture_stdout.1).unwrap_or_exit(PIPE_END_CLOSE_ERR, 4);
                }
                // if !stderr_redirected {
                //     close(fds_capture_stderr.0).unwrap_or_exit(PIPE_END_CLOSE_ERR, 4);
                //     dup2(fds_capture_stderr.1, 2).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                //     close(fds_capture_stderr.1).unwrap_or_exit(PIPE_END_CLOSE_ERR, 4);
                // }
            }

            //TODO 2: Check for builtin commands

            //TODO: Load in env vars
            //TODO: Search in path
            let c_cmd = CString::new(cmd.cmd.as_str())
                .unwrap_or_exit("oyster: cstring error converting command", 5);
            let args: Vec<CString> = cmd.args.into_iter()
                .map(|arg| {
                    CString::new(arg.as_str())
                        .unwrap_or_exit("oyster: cstring error parsing command arguments", 5)
                }).collect();
            let c_args: Vec<&CStr> = args.iter().map(|arg| arg.as_c_str()).collect();
            match execvp(&c_cmd, &c_args) {
                Ok(_) => {}
                Err(e) => {
                    match e {
                        Error::Sys(Errno::ENOEXEC) => {
                            eprintln!("oyster: exec format error");
                        }
                        Error::Sys(Errno::ENOENT) => {
                            eprintln!("oyster: command {} not found", cmd.cmd);
                        }
                        Error::Sys(Errno::EACCES) => {
                            eprintln!("oyster: permission denied");
                        }
                        _ => {
                            eprintln!("oyster: error: {:?}", e);
                        }
                    }
                }
            }

            process::exit(1);
        }
        ForkResult::Parent{child,..} => {
            if !params.capture_output && params.isatty && idx == 0 {
                *pgid = child;
                if !params.background {
                    *term_given = shell::give_terminal_to(child)?;
                }
            }

            match setpgid(child, *pgid) {
                Ok(()) => {}
                Err(e) => { 
                    eprintln!("Could not set child pgid from parent: {}", e); 
                    return Err(e);
                }
            }

            if idx < pipes_count {
                let fds = pipes[idx];
                match close(fds.1) {
                    Ok(()) => {},
                    Err(e) => {
                        eprintln!("error {}: could not close pipe", e);
                        return Err(e);
                    }
                }
            }

            if idx == pipes_count && params.capture_output {
                close(fds_capture_stdout.1)?;
                close(fds_capture_stderr.1)?;
                let mut stdoutfd = unsafe {File::from_raw_fd(fds_capture_stdout.0)};
                let mut sout = String::new();
                stdoutfd.read_to_string(&mut sout).unwrap();
                *results = CommandResult {
                    status: 0,
                    stdout: sout,
                    stderr: String::new(),
                }
            }

            let child_pid: i32 = child.into();

            return Ok(child_pid);
        }
    }
}