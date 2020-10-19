use std::os::unix::io::RawFd;
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
    dup2, 
    close,
    ForkResult
};
use nix::Result;
use nix::Error;
use nix::errno::Errno;

use crate::shell;

//placeholder - move to job control
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};

//placeholder imports for bugfixing
use nix::unistd::execvp;

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
    
    //TODO: Handle expansion logic here
    //* Command substitution cannot contain redirections:
    //* Function will return an error if it does
    
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

    for cmd in &job.cmds {

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

        children.push(childpid);
        idx += 1;
    }

    //placeholder code - waiting will be moved to job control
    for childpid in children {
        match waitpid(Pid::from_raw(childpid), Some(WaitPidFlag::WUNTRACED))? {
            WaitStatus::Exited(pid, status) => {
                println!("process {} exited with status {}", pid, status);
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
    cmd: &Cmd, 
    idx: usize, 
    pgid: &mut Pid,
    pipes: &Vec<(RawFd, RawFd)>,
    params: CommandParams,
    term_given: &mut bool,
    // for capturing output
    results: &mut CommandResult,) -> Result<i32> {

    // Pre: Create pipes to capture output (when doing command expansion)
    //*Fork!
    // 
    //If in child: ========================================
    // 0. Set the pgid
    //
    // 1. If idx > 0 grab the appropriate pipe from pipes
    //    Hook up stdin (0) to pipe using dup2():
    //
    //    dup2(pipefd.0, 0)
    //    close(pipefd.0)
    //
    // 2. If idx < pipes.len() hook up stdout to pipes:
    //    dup2(pipefd.1, 1)
    //    close(pipefd.1)
    //
    // 3. Process redirects and change file descriptors as necessary
    //    TODO: Create a function to create a raw fd from a filename
    //    Use OpenOptions to set append or truncate
    //
    // 4. If output needs to be captured, redirect stdout to capture fds
    //
    // 5. Check if any commands are builtin, and exec them as necessary
    //
    // 6. Load in environment variables and convert for compatibility
    //
    // 7. Find program in path, print error and exit if not found
    //
    // 8. Execute! (execve with args and env)
    //
    //If in parent: ========================================
    //
    // 1. Get child's PID
    //
    // 2. Give terminal to child 
    //    (if on first command, capture output is off and isatty)
    //
    // 3. Close any open file descriptors
    //    Close write ends of the stdout/stderr capture pipes
    //    If on the second last command (last command spawned) and capturing output,
    //    Read from the stdout/stderr descriptors into the strings
    //
    // 4. Close the read ends of the stdout/stderr descriptors
    //
    // 5. Return child's pid

    let mut fds_capture_stdout = pipe()?;
    let mut fds_capture_stderr = pipe()?;

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
                    .unwrap_or_exit("oyster: failed to set pgid", 2);
            } else {
                setpgid(Pid::from_raw(0), *pgid)
                    .unwrap_or_exit("oyster: failed to set pgid", 2);
            }

            //connecting up pipes for commands to read from
            if idx > 0 { //not the first command
                let fds = pipes[idx - 1];
                dup2(fds.0, 0).unwrap_or_exit("oyster: failed to duplicate file descriptor", 3);
                close(fds.0).unwrap_or_exit("oyster: failed to connect pipes", 4);
            }

            //connecting up pipes for commands to write to
            if idx < pipes_count {
                let fds = pipes[idx];
                dup2(fds.1, 1).unwrap_or_exit("oyster: failed to connect pipes", 4);
            }

            //TODO 1: Handle redirects

            //TODO 2: Check for builtin commands

            //TODO 3: Redirecting output to output capture

            //TODO: Load in env vars
            //TODO: Search in path
            let cmdstring = cmd.cmd.clone();
            let c_cmd = CString::new(cmdstring.as_str())
                .unwrap_or_exit("oyster: cstring error converting command", 5);
            let args: Vec<CString> = cmd.args.clone().into_iter()
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
            match setpgid(child, *pgid) {
                Ok(()) => {}
                Err(e) => { 
                    eprintln!("Could not set child pgid from parent: {}", e); 
                    return Err(e);
                }
            }

            let child_pid: i32 = child.into();

            return Ok(child_pid);
        }
    }
}