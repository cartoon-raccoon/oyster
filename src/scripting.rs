//use std::collections::HashMap;

use crate::types::{Job, ShellError};
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
#[derive(Debug, Clone)]
pub enum Construct {
    /// Represents a `for` loop
    For {
        loop_var: String,
        loop_over: Vec<String>,
        code: Vec<Box<Construct>>,
    },
    /// Represents an `if/elif/else` statement.
    #[allow(dead_code)]
    If {
        conditions: Vec<Job>,
        code: Vec<Box<Construct>>
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
                let details = raw.remove(0).cmds.remove(0);
                if details.args.len() < 5 {
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
                unimplemented!()
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
                if code.is_empty() {
                    return Ok(0)
                }
                for condition in conditions.into_iter() {
                    //for every if and elif
                    let to_exec = code.remove(0);
                    if exec(shell, condition, false, false)?.status == 0 {
                        return to_exec.execute(shell)
                    }
                }
                //we have reached the else statement
                //at this point there should only be one item left in code
                //if code is empty, there is no else clause
                if code.len() == 1 {
                    let to_exec = code.remove(0);
                    return to_exec.execute(shell)
                } else if code.len() > 1 {
                    return Err(
                        ShellError::from("error: code and condition mismatch")
                    )
                }
                Ok(0) //code is empty
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

fn split_on_same_scope(raw: Vec<Job>) -> Vec<Vec<Job>> {
    let mut to_return = Vec::new();
    let mut buffer = Vec::new();
    let mut nesting_level: usize = 0;
    for job in raw {
        match job.cmds[0].cmd.1.as_str() {
            "for" => {
                buffer.push(job);
                nesting_level += 1;
            }
            "if" => { 
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
