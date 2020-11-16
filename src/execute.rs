use std::path::Path;

use nix::unistd::getpgid;

use crate::types::{
    Job, 
    Cmd,
    Exec, 
    Quote,
    CommandResult,
    ShellError,
    ExecType,
};
use crate::core;
use crate::shell::{
    self, Shell
};
use crate::expansion::{
    expand_braces,
    expand_variables,
    expand_tilde,
    substitute_commands,
};
use crate::builtins::*;
use crate::scripting::*;

/// High level control of all jobs. Conditional execution is handled here.
/// Parses tokens into jobs, performs expansion and executes them.
pub fn execute_jobs(
    shell: &mut Shell, 
    mut jobs: Vec<Job>, 
    capture: bool
) -> Result<(i32, String), ShellError> {

    let mut captured = String::new();
    let mut execif: Option<Exec>;
    let mut result = CommandResult::new();

    if jobs[0].cmds[0].cmd.0 == Quote::NQuote {
        if jobs[0].cmds[0].cmd.1 == "func" {
            if let Some(job) = jobs.last() {
                if job.cmds[0].cmd.1 != "endfn" {
                    return Err(
                        ShellError::from("oyster: cannot parse function")
                    )
                }
            }
            let mut func = jobs.remove(0);
            let mut params_count = None;
            if func.cmds[0].args.len() > 2 {
                params_count = match func.cmds[0].args[2].1.parse::<usize>() {
                    Ok(int) => Some(int),
                    Err(_) => {
                        return Err(
                            ShellError::from("oyster: invalid function params count")
                        )
                    }
                };
            }
            let funcname = func.cmds.remove(0)
                           .args.remove(1).1;
            jobs.pop();
            shell.insert_func(&funcname, jobs, params_count);
            return Ok((0, String::new()))
        }
    }

    //if execute_jobs is called from scripting,
    //this should not return any shell constructs
    let jobs = extract_constructs(jobs)?;

    for job in jobs {
        match job {
            ExecType::Job(job) => {
                if job.cmds[0].cmd.0 == Quote::NQuote
                && job.cmds[0].cmd.1 == "return" {
                    return Ok((0, String::new()))
                }
                execif = job.execnext;
                if let Some(execcond) = execif {
                    match execcond {
                        Exec::And => { //continue if last job succeeded
                            if job.cmds[0].cmd.1.ends_with("()") {
                                result = execute_func(shell, job)?.into();
                            } else {
                                result = execute(shell, job, false, capture)?;
                            }
                            captured.push_str(&result.stdout);
                            if result.status == 0 {
                                continue;
                            } else {
                                return Ok((result.status, captured));
                            }
                        }
                        Exec::Or => { //continue if last job failed
                            if job.cmds[0].cmd.1.ends_with("()") {
                                result = execute_func(shell, job)?.into();
                            } else {
                                result = execute(shell, job, false, capture)?;
                            } 
                            captured.push_str(&result.stdout);
                            if result.status != 0 {
                                continue;
                            } else {
                                return Ok((result.status, captured));
                            }
                        }
                        Exec::Consec => { //unconditional execution
                            if job.cmds[0].cmd.1.ends_with("()") {
                                result = execute_func(shell, job)?.into();
                            } else {
                                result = execute(shell, job, false, capture)?;
                            }
                            captured.push_str(&result.stdout);
                            continue;
                        }
                        Exec::Background => { //run jobs asynchronously
                            if job.cmds[0].cmd.1.ends_with("()") {
                                result = execute_func(shell, job)?.into();
                            } else {
                                result = execute(shell, job, false, capture)?;
                            }
                            captured.push_str(&result.stdout);
                            continue;
                        }
                    }
                } else { //if is None; this should only occur on the last job
                    if job.cmds[0].cmd.1.ends_with("()") {
                        result = execute_func(shell, job)?.into();
                    } else {
                        result = execute(shell, job, false, capture)?;
                    }
                    captured.push_str(&result.stdout);
                }
            }
            ExecType::Script(script) => {
                //println!("got script: {:?}", script);
                let script = Construct::build(shell, script)?;
                result.status = script.execute(shell)?;
            }
        }
        
    }
    Ok((result.status, captured))
}

/// Lower level control. Executes single pipeline.
/// Checks for builtins without pipeline
pub fn execute(
    shell: &mut Shell, 
    job: Job, 
    background: bool,
    capture: bool,
) -> Result<CommandResult, ShellError> {

    let mut cmds: Vec<Cmd> = Vec::new();
    for cmd in job.cmds {
        cmds.push(Cmd::from_tokencmd(shell, cmd)?)
    }

    if cmds.len() < 1 {
        return Err(
            ShellError::from("oyster: empty job")
        )
    }

    if cmds.len() == 1 && !capture { //no pipeline
        let mut cmd = cmds[0].clone();
        if shell::assign_variables(shell, &mut cmd.cmd) {
            return Ok(CommandResult::new());
        }
        if Path::new(&cmd.cmd).is_dir() {
            let status = cd::run(shell, cmd, true);
            return Ok(CommandResult::from_status(status))
        }
        match cmd.cmd.as_str() {
            "cd" => {
                let status = cd::run(shell, cmd, false);
                return Ok(CommandResult::from_status(status));
            }
            "bg" => {
                let status = bg::run(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "fg" => {
                let status = fg::run(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "jobs" => {
                let status = jobs::run(shell, cmd);
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
            "let" => {
                let status = set::run(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "which" => {
                let status = which::run(cmd);
                return Ok(CommandResult::from_status(status));
            }
            "show" => {
                let status = show::run(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "pushd" => {
                let status = dirstack::pushd(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "popd" => {
                let status = dirstack::popd(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "dirs" => {
                let status = dirstack::dirs(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            "eval" => {
            }
            "source" => {
            }
            "export" => {
                let status = export::run(cmd);
                return Ok(CommandResult::from_status(status));
            }
            "echo" => {
            }
            "kill" => {
            }
            "exit" => {
                let status = exit::run(shell, cmd);
                return Ok(CommandResult::from_status(status));
            }
            _ => {}
        }
    }
    
    let (given, result) = core::run_pipeline(
        shell, job.id, cmds, background, capture
    )?;
    if given {
        let pgid = getpgid(None)?;
        shell::give_terminal_to(pgid)?;
    }
    Ok(result)
}

fn extract_constructs(jobs: Vec<Job>) -> Result<Vec<ExecType>, ShellError> {
    let mut to_return = Vec::new();
    let mut buffer = Vec::new();

    //tracks the top level of the shell construct
    let mut in_construct = false;
    let mut nesting_level: isize = 0;

    for job in jobs {
        if job.cmds[0].cmd.0 == Quote::NQuote {
            match job.cmds[0].cmd.1.as_str() {
                "for" | "while" | "if" => {
                    if !in_construct {
                        in_construct = true; 
                    }
                    nesting_level += 1;
                }
                "done" | "end" => {
                    nesting_level -= 1;
                }
                _ => {}
            }
        }
        if in_construct {
            buffer.push(job);
            if nesting_level == 0 {
                to_return.push(ExecType::Script(buffer.clone()));
                buffer.clear();
                in_construct = false;
            }
        } else {
            to_return.push(ExecType::Job(job));
        }
    }
    if nesting_level > 0 {
        return Err(ShellError::from("oyster: parse error in script"))
    }
    Ok(to_return)
}

fn execute_func(shell: &mut Shell, job: Job) -> Result<(i32, String), ShellError> {
    let func_to_exec =
        job.cmds[0].cmd.1.replace("()", "");
    let mut func_args = Vec::<String>::new();
    if job.cmds[0].args.len() > 1 {
        func_args = Vec::new();
        for (quote, mut string) in job.cmds[0].args[1..].to_vec() {
            match quote {
                Quote::NQuote => {
                    expand_variables(shell, &mut string);
                    expand_tilde(shell, &mut string);
                    func_args.push(string);
                }
                Quote::DQuote => {
                    expand_variables(shell, &mut string);
                    string = substitute_commands(shell, &string)?;
                    func_args.push(string);
                }
                Quote::BQuote => {
                    expand_variables(shell, &mut string);
                    func_args.extend(substitute_commands(shell, &string)?
                        .split_whitespace().map(|s| s.to_string())
                        .collect::<Vec<String>>()
                    );
                }
                Quote::CmdSub => {
                    func_args.extend(substitute_commands(shell, &string)?
                        .split_whitespace().map(|s| s.to_string())
                        .collect::<Vec<String>>()
                    );
                }
                Quote::CBrace => {
                    func_args.extend(expand_braces(shell, string)?);
                }
                Quote::NmSpce => {
                    func_args.push(string);
                }
                Quote::SQuote => {
                    func_args.push(string);
                }
                Quote::SqBrkt => {
                    func_args.push(shell::eval_sqbrkt(shell, string.clone())?.to_string());
                }
            }
        }
    }
    return shell.execute_func(&func_to_exec, func_args)
}