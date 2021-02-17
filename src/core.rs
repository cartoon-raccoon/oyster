use std::fs::File;
use std::io::Read;
use std::os::unix::io::{
    RawFd,
    FromRawFd,
};
use std::env;
use std::process;
use std::ffi::{CString, CStr};

use nix::unistd::{
    Pid,
    getpid,
    setpgid,
    isatty,
    fork, 
    pipe, 
    execve, 
    dup, dup2, 
    close,
    ForkResult
};
use nix::sys::signal::{signal, Signal, SigHandler};
use nix::Error;
use nix::errno::Errno;

use crate::types::*;
use crate::jobc;
use crate::shell::{self, Shell};
use crate::builtins::*;

/// Even lower level, it deconstructs the job
/// and passes raw parameters to the final function.
pub fn run_pipeline(
    shell: &mut Shell, 
    job_id: i32,
    cmds: Vec<Cmd>,
    background: bool, 
    capture: bool) -> Result<(bool, CommandResult), ShellError> {
    
    //defaults to return
    let mut term_given = false;
    let mut cmdresult = CommandResult::new();

    //making vec of pipes
    let mut pipes = Vec::new();
    for _ in 0..cmds.len() - 1 {
        pipes.push(pipe()?);
    }

    let isatty = isatty(1)?;

    let mut pgid = Pid::from_raw(0);
    let mut idx: usize = 0;
    let mut children = Vec::new();

    for cmd in cmds {

        let params = CommandParams{
            isatty: isatty,
            background: background,
            capture_output: capture, //is used in command expansion only
            env: shell.env().clone(),
        };

        let childpid = run_command(
            job_id,
            cmd, 
            idx, 
            &mut pgid,
            &pipes,
            params,
            shell,
            &mut term_given,
            &mut cmdresult,
        )?;
        if childpid > 0 && !background {
            children.push(childpid);
        }
        idx += 1;
    }

    let mut status: i32 = 0;
    for childpid in children {
        status = jobc::wait_on_job(
            shell, 
            pgid, 
            Pid::from_raw(childpid), 
            true
        );
        cmdresult.status = status;
    }
    
    for pipe in pipes {
        close(pipe.0)?;
        match close(pipe.1) {
            Ok(_) => {},
            Err(_) => {}
        }
    }

    if background {
        if let Some(job) = shell.get_job_by_pgid(pgid) {
            eprintln!("[{}] {} {}", job.id, job.pgid, job.firstcmd);
        }
    }
    
    if status == STOPPED {
        jobc::mark_job_as_stopped(shell, job_id);
    }
    Ok((term_given, cmdresult))
}

/// This is one deep-ass core function.
fn run_command(
    id: i32,
    cmd: Cmd, 
    idx: usize, 
    pgid: &mut Pid,
    pipes: &Vec<(RawFd, RawFd)>,
    params: CommandParams,
    shell: &mut Shell,
    term_given: &mut bool,
    // for capturing output
    results: &mut CommandResult,) -> Result<i32, ShellError> {

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

            unsafe {
                signal(Signal::SIGINT, SigHandler::SigDfl)?;
                signal(Signal::SIGQUIT, SigHandler::SigDfl)?;
                signal(Signal::SIGTSTP, SigHandler::SigDfl)?;
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

            if cmd.pipe_stderr {
                if idx < pipes_count {
                    let fds = pipes[idx];
                    dup2(fds.1, 2).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                } else if !params.capture_output {
                    let fd = dup(2).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                    dup2(fd, 2).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
                }
            }

            let mut stdout_redirected = false;
            //let mut stderr_redirected = false;

            for redirect in &cmd.redirects {
                
                if redirect.0 == "2" && redirect.2 == "&1" {
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
                } else if redirect.2 == "0" && redirect.1 == Redirect::FromStdin {
                    let fd = shell::open_file_as_fd(&redirect.0);
                    dup2(fd, 0).unwrap_or_exit(FD_DUPLICATE_ERR, 3);
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

            match cmd.cmd.as_str() {
                "cd" => {
                    let status = cd::run(shell, cmd, false);
                    process::exit(status);
                }
                "bg" => {
                    let status = bg::run(shell, cmd);
                    process::exit(status);                
                }
                "fg" => {
                    let status = fg::run(shell, cmd);
                    process::exit(status);                
                }
                "jobs" => {
                    let status = jobs::run(shell, cmd);
                    process::exit(status);
                }
                "alias" => {
                    let status = alias::set(shell, cmd);
                    process::exit(status);
                }
                "unalias" => {
                    let status = alias::unset(shell, cmd);
                    process::exit(status);
                }
                "let" => {
                    let status = set::run(shell, cmd);
                    process::exit(status);
                }
                "which" => {
                    let status = which::run(shell, cmd);
                    process::exit(status);
                }
                "show" => {
                    let status = show::run(shell, cmd);
                    process::exit(status);
                }
                "pushd" => {
                    let status = dirstack::pushd(shell, cmd);
                    process::exit(status);
                }
                "popd" => {
                    let status = dirstack::popd(shell, cmd);
                    process::exit(status);
                }
                "dirs" => {
                    let status = dirstack::dirs(shell, cmd);
                    process::exit(status);
                }
                "eval" => {
                }
                "source" => {
                }
                "export" => {
                    let status = export::run(cmd);
                    process::exit(status);
                }
                "echo" => {
                }
                "kill" => {
                }
                "exit" => {
                    let status = exit::run(shell, cmd);
                    process::exit(status);
                }
                _ => {}
            }

            let c_cmd = if !cmd.cmd.contains("/") {
                CString::new(
                shell.search_in_path(&cmd.cmd)
                .unwrap_or_exit(&format!("oyster: command {} not found", cmd.cmd), 1)
                .to_str().unwrap_or_exit("oyster: osstring conversion error", 5))
                .unwrap_or_exit("oyster: cstring error converting command", 5)
            } else {
                CString::new(cmd.cmd)
                .unwrap_or_exit("oyster: cstring error converting command", 5)
            };
            let args: Vec<CString> = cmd.args.into_iter()
                .map(|arg| {
                    CString::new(arg.as_str())
                        .unwrap_or_exit("oyster: cstring error parsing command arguments", 5)
            }).collect();
            let envs: Vec<CString> = env::vars().map(|(key, var)| {
                CString::new(format!("{}={}", key, var))
                    .unwrap_or_exit("oyster: cstring error parsing env vars", 5)
            }).collect();
            let c_args: Vec<&CStr> = args.iter().map(|arg| arg.as_c_str()).collect();
            let c_envs: Vec<&CStr> = envs.iter().map(|env| env.as_c_str()).collect();
            match execve(&c_cmd, &c_args, &c_envs) {
                Ok(_) => {}
                Err(e) => {
                    match e {
                        Error::Sys(Errno::ENOEXEC) => {
                            eprintln!("oyster: exec format error");
                        }
                        Error::Sys(Errno::ENOENT) => {
                            eprintln!("oyster: no such file in directory");
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
                    return Err(e.into());
                }
            }

            if idx < pipes_count {
                let fds = pipes[idx];
                match close(fds.1) {
                    Ok(()) => {},
                    Err(e) => {
                        eprintln!("error {}: could not close pipe", e);
                        return Err(e.into());
                    }
                }
            }

            if params.isatty && !params.capture_output {
                shell.add_cmd_to_job(
                    id, 
                    child, 
                    *pgid, 
                    cmd.cmd.clone(),
                    params.background
                );
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