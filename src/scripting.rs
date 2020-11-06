//use std::collections::HashMap;

use crate::types::{
    Job, 
    ShellError, 
    Exec, 
    TokenCmd,
    Quote,
};
use crate::shell::Shell;
use crate::execute::{
    execute_jobs,
    execute as exec,
};
//TODO: Implement scoping; rn all variables have global scope

/// # Construct
/// 
/// A recursive type that forms an abstract syntax tree.
/// 
/// The `For` and `If` variants are branches that store the enum
/// itself and the `Code` variant is the leaf (base case) that
/// stores the actual commands to execute.
/// This allows `Construct` to recursively build itself
/// and then recursively execute itself.
/// Each variant implements its own behaviour, but ultimately
/// the branch variants call `execute()` on their own code.
/// 
/// The leaf variant evaluates its own jobs and returns
/// the exit status of the last command.
#[derive(Debug, Clone, PartialEq)]
pub enum Construct {
    /// Represents a `for` loop
    For {
        loop_var: String,
        loop_over: Vec<String>,
        code: Vec<Box<Construct>>,
    },
    /// Represents an `if/elif/else` statement.
    If {
        conditions: Vec<(Job, bool)>,
        code: Vec<Vec<Box<Construct>>>
    },
    /// The base case for everything.
    ///
    /// Does not contain itself.
    Code(Vec<Job>),
}

impl Construct {
    /// Recursively builds the AST from a raw Vec of jobs.
    pub fn build(mut raw: Vec<Job>) -> Result<Self, ShellError> {
        if raw.len() < 1 {
            return Err(ShellError::from("error: empty script"))
        }
        let len = raw.len();
        //only need to split after matching, because build() receives
        //a single construct that may contain many constructs on the same scope
        //but deeper inside
        if raw[0].execnext != Some(Exec::Consec) {
            return Err(
                ShellError::from("oyster: invalid delimiter")
            )
        }
        match raw[0].cmds[0].cmd.1.as_str() {
            "for" => {
                if let Some(last) = raw.iter().last() {
                    //last.cmds should not be empty
                    if last.cmds[0].cmd.1 != "done" {
                        return
                        Err(
                            ShellError::from("oyster: could not parse script")
                        )
                    }
                }
                raw.remove(len - 1);
                let mut details = raw.remove(0).cmds.remove(0);
                if details.args.len() < 4 {
                    return
                    Err(
                        ShellError::from("oyster: could not parse for loop")
                    )
                }
                if details.args[2].1 != "in" {
                    return
                    Err(
                        ShellError::from("oyster: invalid for loop syntax")
                    )
                }
                if details.args[3].0 == Quote::SqBrkt {
                    let (_quote, to_expand) = details.args.remove(3);
                    details.args.extend(expand_sqbrkt_range(to_expand)?);
                }
                //TODO: match on Quote type
                let coden = split_on_same_scope(raw);
                let mut final_code = Vec::new();
                for construct in coden {
                    final_code.push(Box::new(Construct::build(construct)?));
                }
                Ok(Construct::For {
                    loop_var: details.args[1].1.to_string(),
                    loop_over: details.args[3..].to_vec()
                        .into_iter()
                        .map(|tuple| tuple.1)
                        .collect(),
                    code: final_code,
                })
            }
            "if" => {
                let mut final_code = Vec::new();
                let mut conditions = Vec::new();
                for (condition, jobs) in split_on_branches(raw)? {
                    let mut code = Vec::new();
                    if let Some(condition) = condition {
                        conditions.push(condition);
                    }
                    for job in split_on_same_scope(jobs) {
                        code.push(Box::new(Construct::build(job)?));
                    }
                    final_code.push(code);
                }
                Ok(Construct::If {
                    conditions: conditions,
                    code: final_code,
                })
            }
            n@ "done" | n@ "end" | n@ "elif" | n@ "else" => {
                return Err(
                    ShellError::from(format!("oyster: parse error near {}", n))
                )
            }
            _ => {
                return Ok(Construct::Code(raw))
            }
        }
    }
    /// Recursively executes itself, consuming itself as it goes.
    pub fn execute(
        self,
        shell: &mut Shell,
    ) -> Result<i32, ShellError> {

        match self {
            Construct::For {loop_var, loop_over, code} => {
                let mut status: i32 = 0;

                for item in loop_over {
                    shell.add_variable(&loop_var, &item);
                    //* clone here slows things down a lot
                    let code2 = code.clone();
                    for block in code2 {
                        status = block.execute(shell)?;
                    }
                }
                Ok(status)
            }
            Construct::If {conditions, mut code} => {
                let mut status: i32 = 0;
                if code.is_empty() {
                    return Ok(0)
                }
                //note: this isn't particularly efficient
                for (condition, execif) in conditions {
                    //for every if and elif
                    let to_exec = code.remove(0);
                    //if execif is true, execute the code block
                    if exec(shell, condition, false, false)?.status == 0 && execif {
                        for job in to_exec {
                            status = job.execute(shell)?;
                        }
                        return Ok(status)
                    }
                }
                //we have reached the else statement
                //at this point there should only be one item left in code
                //if code is empty, there is no else clause
                if code.len() == 1 {
                    let to_exec = code.remove(0);
                    for job in to_exec {
                        status = job.execute(shell)?;
                    }
                } else if code.len() > 1 {
                    return Err(
                        ShellError::from("error: code and condition mismatch")
                    )
                }
                Ok(status) //code is empty
            }
            //Base case
            Construct::Code(code) => {
                // for (key, value) in vars {
                //     shell.add_variable(&key, &value);
                // }
                let (status, _cap) = execute_jobs(shell, code, false)?;
                Ok(status)
            }
        }
    }
}

fn expand_sqbrkt_range(brkt: String) -> Result<Vec<(Quote, String)>, ShellError> {
    let range: Vec<&str> = brkt.split("..").filter(
        |string| !string.is_empty()
    ).collect();
    if range.len() != 2 {
        return Err(
            ShellError::from("oyster: error expanding range")
        )
    }
    let mut numeric = Vec::new();
    for number in range {
        match number.parse::<i32>() {
            Ok(int) => {
                numeric.push(int);
            }
            Err(_) => {
                return Err(
                    ShellError::from("oyster: non-integer character in range")
                )
            }
        }
    }
    let (mut start, end) = (numeric[0], numeric[1]);
    let mut to_return = Vec::new();
    while start < end {
        to_return.push((Quote::NQuote, start.to_string()));
        start += 1;
    }
    Ok(to_return)
}

/// Splits a single scope into its individual constructs
fn split_on_same_scope(raw: Vec<Job>) -> Vec<Vec<Job>> {
    let mut to_return = Vec::new();
    let mut buffer = Vec::new();
    let mut nesting_level: usize = 0;
    for job in raw {
        match job.cmds[0].cmd.1.as_str() {
            "for" | "if" => {
                buffer.push(job);
                nesting_level += 1;
            }
            "done" | "end" => {
                buffer.push(job);
                nesting_level -= 1;
                if nesting_level == 0 {
                    to_return.push(buffer.clone());
                    buffer.clear();
                }
            }
            _ => {
                buffer.push(job);
            }
        }
    }
    if buffer.len() > 0 {
        to_return.push(buffer);
    }
    to_return
}

/// Splits an if statement into if/elif/else blocks
//TODO: optimise this (i.e. remove clones `urgh`)
//TODO2: Fix ! condition
fn split_on_branches(raw: Vec<Job>) 
             //condition    codeblock  
-> Result<Vec<(Option<(Job, bool)>, Vec<Job>)>, ShellError> {
    let mut to_return = Vec::<(Option<(Job, bool)>, Vec<Job>)>::new();
    let mut buffer = Vec::new();
    let mut nesting_level: isize = -1;
    let mut condition = None;
    for job in raw {
        match job.cmds[0].cmd.1.as_str() {
            "for" => {
                buffer.push(job);
                nesting_level += 1;
            }
            "if" => { 
                nesting_level += 1;
                if nesting_level == 0 {
                    let command = if job.cmds[0].args[1].1.starts_with("!") {
                        (
                            job.cmds[0].args[1].0, 
                            job.cmds[0].args[1].1[1..].to_string()
                        )
                    } else {
                        job.cmds[0].args[1].clone()
                    };
                    condition = Some((Job {
                        cmds: vec![TokenCmd {
                            cmd: command,
                            args: job.cmds[0].args[1..].to_vec(),
                            redirects: job.cmds[0].redirects.clone(),
                            pipe_stderr: job.cmds[0].pipe_stderr,
                        }],
                        execnext: job.execnext,
                        id: job.id,
                        },
                        !job.cmds[0].args[1].1.starts_with("!"))
                    );
                } else {
                    buffer.push(job);
                }
            }
            "elif" => {
                if nesting_level == 0 {
                    if job == Job::default() {
                        return Err(
                            ShellError::from(
                                "oyster: syntax error near `elif`"
                            )
                        )
                    }
                    to_return.push(
                        (condition.clone(), 
                        buffer.clone()));
                    buffer.clear();
                    let command = if job.cmds[0].args[1].1.starts_with("!") {
                        (
                            job.cmds[0].args[1].0, 
                            job.cmds[0].args[1].1[1..].to_string()
                        )
                    } else {
                        job.cmds[0].args[1].clone()
                    };
                    condition = Some((Job {
                        cmds: vec![TokenCmd {
                            cmd: command,
                            args: job.cmds[0].args[1..].to_vec(),
                            redirects: job.cmds[0].redirects.clone(),
                            pipe_stderr: job.cmds[0].pipe_stderr,
                        }],
                        execnext: job.execnext,
                        id: job.id,
                        },
                        !job.cmds[0].args[1].1.starts_with("!"))
                    );
                } else {
                    buffer.push(job);
                }
            }
            "else" => {
                if nesting_level == 0 {
                    to_return.push(
                        (condition.clone(), 
                        buffer.clone()));
                    buffer.clear();
                    condition = None;
                } else {
                    buffer.push(job);
                }
            }
            "done" => {
                buffer.push(job);
                nesting_level -= 1;
            }
            "end" => {
                if nesting_level == 0 {
                    to_return.push((condition.clone(), buffer.clone()));
                    buffer.clear();
                }
                nesting_level -= 1;
            }
            _ => {
                buffer.push(job);
            }
        }
    }
    if nesting_level > 0 {
        return Err(
            ShellError::from("oyster: parse error in if statement")
        )
    }
    //println!("{:?}", to_return);
    
    Ok(to_return)
}