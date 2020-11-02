use std::collections::HashMap;

use crate::types::{Job, ShellError};
use crate::shell::Shell;
use crate::execute::execute_jobs;

pub trait ShellConstruct {
    fn execute(
        &mut self, 
        shell: &mut Shell, 
        vars: &mut HashMap<String, String>
    ) -> Result<i32, ShellError>;
}

//impl ShellConstruct
pub struct CodeBlock {
    jobs: Vec<Job>,
}

impl ShellConstruct for CodeBlock {
    fn execute(
        &mut self, 
        shell: &mut Shell, 
        vars: &mut HashMap<String, String>) 
        -> Result<i32, ShellError> {
        
        for (key, value) in vars {
            shell.add_variable(key, value);
        }
        let (status, cap) = execute_jobs(shell, self.jobs.clone(), false)?;
        Ok(status)
    }
}

//impl ShellConstruct
pub struct ForLoop {
    loop_var: String,
    loop_over: Vec<String>,
    codeblocks: Vec<Box<dyn ShellConstruct>>
}

impl ShellConstruct for ForLoop {
    fn execute(
        &mut self, 
        shell: &mut Shell, 
        vars: &mut HashMap<String, String>) 
        -> Result<i32, ShellError> {

        let mut status: i32 = 0;
        //TODO: fix the fucking clones
        for item in &mut self.loop_over {
            let entry = vars.entry(self.loop_var.clone())
                .or_insert(self.loop_var.clone());
            *entry = item.clone();
            for code in &mut self.codeblocks {
                status = code.execute(shell, vars)?;
            }
        }
        Ok(status)
    }
}

impl ForLoop {
    pub fn from(jobs: Vec<Job>) {

    }
    pub fn execute(&mut self, shell: &mut Shell) {

    }
}

pub struct WhileLoop {
    //terminator: ???
    jobs: Vec<Job>
}

impl WhileLoop {
    pub fn from(jobs: Vec<Job>) {

    }
    pub fn execute(&mut self, shell: &mut Shell) {

    }
}

pub struct If {
    //how to express this?
}

pub struct Switch {

}