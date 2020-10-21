use std::error::Error;

use nix::unistd::getpgid;

use crate::types::{Token, Job, Exec, CommandResult};
use crate::parser::Lexer;
use crate::core;
use crate::shell::{
    self, Shell
};
use crate::builtins::{
    cd,
    set,
    alias,
};

/// High level control of all jobs. Conditional execution is handled here.
/// Parses tokens into jobs, performs expansion and executes them.
pub fn execute_jobs(
    shell: &mut Shell, 
    tokens: Vec<Token>, 
    capture: bool
) -> Result<(i32, String), Box<dyn Error>> {
    
    let jobs = Lexer::parse_tokens(shell, tokens)?;
    //println!("{:?}", jobs);

    let mut captured = String::new();
    let mut execif: Option<Exec>;

    for job in jobs {
        execif = job.execnext;
        if let Some(execcond) = execif {
            match execcond {
                Exec::And => { //continue if last job succeeded
                    let result = execute(shell, job, false, capture)?;
                    captured.push_str(&result.stdout);
                    if result.status == 0 {
                        continue;
                    } else {
                        return Ok((result.status, captured));
                    }
                }
                Exec::Or => { //continue if last job failed
                    let result = execute(shell, job, false, capture)?; 
                    captured.push_str(&result.stdout);
                    if result.status != 0 {
                        continue;
                    } else {
                        return Ok((result.status, captured));
                    }
                }
                Exec::Consec => { //unconditional execution
                    let result = execute(shell, job, false, capture)?;
                    captured.push_str(&result.stdout);
                    continue;
                }
                Exec::Background => { //run jobs asynchronously
                    let result = execute(shell, job, true, capture)?;
                    captured.push_str(&result.stdout);
                    continue;
                }
            }
        } else { //if is None; this should only occur on the last job
            let result = execute(shell, job, false, capture)?;
            return Ok((result.status, captured));
        }
    }
    Ok((0, String::new()))
}

/// Lower level control. Executes single pipeline.
/// Checks for builtins without pipeline
pub fn execute(
    shell: &mut Shell, 
    job: Job, 
    background: bool,
    capture: bool,
) -> Result<CommandResult, Box<dyn Error>> {

    if job.cmds.len() == 1 { //no pipeline
        let mut cmd = job.cmds[0].clone();
        if shell::assign_variables(shell, &mut cmd.cmd) {
            return Ok(CommandResult::new());
        }
        match cmd.cmd.as_str() {
            "cd" => {
                let status = cd::run(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "alias" => {
                let status = alias::set(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "unalias" => {
                let status = alias::unset(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "set" => {
                let status = set::run(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "which" => {
            }
            "eval" => {
            }
            "source" => {
            }
            "export" => {
            }
            "echo" => {
            }
            "kill" => {
            }
            "exit" => {
            }
            _ => {}
        }
    }
    
    let (given, result) = core::run_pipeline(shell, job, background, capture)?;
    if given {
        let pgid = getpgid(None)?;
        shell::give_terminal_to(pgid)?;
    }
    Ok(result)
}