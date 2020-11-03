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
pub enum Construct<'a> {
    /// Represents a `for` loop
    For {
        loop_var: &'a str,
        loop_over: Vec<&'a str>,
        code: Vec<Box<Construct<'a>>>,
    },
    /// Represents an `if/elif/else` statement.
    If {
        conditions: Vec<Job>,
        code: Vec<Box<Construct<'a>>>
    },
    /// The base case for everything.
    ///
    /// Does not contain itself.
    Code(Vec<Job>),
}

impl<'a> Construct<'a> {
    /// Recursively builds the AST from a raw Vec of jobs.
    pub fn build(raw: Vec<Job>) -> Self {
        unimplemented!()
    }
    /// Recursively executes itself, consuming itself as it goes.
    pub fn execute(
        self,
        shell: &mut Shell,
    ) -> Result<i32, ShellError> {

        match self {
            Construct::For {loop_var, loop_over, code} => {
                let mut status: i32 = 0;

                let loop_var2 = loop_var;

                for item in loop_over {
                    shell.add_variable(&loop_var, item);
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
                let (status, cap) = execute_jobs(shell, code, false)?;
                Ok(status)
            }
        }
    }
}
