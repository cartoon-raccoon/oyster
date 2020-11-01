use crate::types::Job;
use crate::shell::Shell;

pub struct ForLoop {
    //loop_over: ???
    jobs: Vec<Job>
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