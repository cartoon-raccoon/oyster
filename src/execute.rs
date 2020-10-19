use std::error::Error;

use nix::unistd::getpgid;

use crate::types::{Token, Job, Exec};
use crate::parser::Lexer;
use crate::core;
use crate::shell::{self, Shell};


/// High level control of all jobs. Conditional execution is handled here.
/// Parses tokens into jobs, performs expansion and executes them.
pub fn execute_jobs(shell: &mut Shell, tokens: Vec<Token>) -> Result<(), Box<dyn Error>> {
    /*
    * Step 1: Match on exec condition
    * Step 2: Pass job to execute()
    */
    let jobs = Lexer::parse_tokens(tokens)?;

    //* perform all expansions here
    //* this will alter the job structs

    let mut execif: Option<Exec>;

    for job in jobs {
        execif = job.execnext;
        if let Some(execcond) = execif {
            match execcond {
                Exec::And => { //continue if last job succeeded
                    if execute(shell, job, false)? {
                        continue;
                    } else {
                        //return error
                    }
                }
                Exec::Or => { //continue if last job failed
                    if !execute(shell, job, false)? {
                        continue;
                    } else {
                        //return error
                    }
                }
                Exec::Consec => { //unconditional execution
                    execute(shell, job, true)?;
                    continue;
                }
                Exec::Background => { //run jobs asynchronously

                }
            }
        } else { //if is None; this should only occur on the last job
            execute(shell, job, false)?;
        }
    }
    Ok(())
}

/// Lower level control. Executes single pipeline.
/// Checks for builtins without pipeline
pub fn execute(shell: &mut Shell, job: Job, background: bool) -> Result<bool, Box<dyn Error>> {

    if job.cmds.len() == 1 { //no pipeline
        match job.cmds[0].cmd.as_str() {
            "cd" => {
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
    
    let (given, result) = core::run_pipeline(shell, job, background, false)?;
    if given {
        let pgid = getpgid(None)?;
        shell::give_terminal_to(pgid)?;
    }
    Ok(true)
}