use std::os::unix::io::RawFd;

use nix::unistd::{
    isatty,
    fork, 
    pipe, 
    execve, 
    dup2, 
    ForkResult
};
use nix::Result;

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

    
    let mut idx: i32 = 0;
    for cmd in &job.cmds {

        let params = CommandParams{
            isatty: isatty,
            background: background,
            capture_output: capture, //is used in command expansion only
            env: shell.env.clone(),
        };

        run_command(
            cmd, 
            idx, 
            &pipes,
            params,
            &mut term_given,
            &mut cmdresult,
        )?;
        idx += 1;
    }
    Ok((term_given, CommandResult::new()))
}

/// This is one deep-ass core function.
fn run_command(
    cmd: &Cmd, 
    idx: i32, 
    pipes: &Vec<(RawFd, RawFd)>,
    params: CommandParams,
    term_given: &mut bool,
    results: &mut CommandResult,) -> Result<i32> {

    // Pre: Create pipes to capture output (when doing command expansion)
    //*Fork!
    // 
    //If in child: ========================================
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

    let forkresult = fork()?;
    match forkresult {
        ForkResult::Child => {
            if idx == 0 {

            }
        }
        ForkResult::Parent{child,..} => {
            let child_pid: i32 = child.into();
        }
    }

    Ok(0)

}