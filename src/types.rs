#[derive(Debug, Clone)]
pub enum Token {
    Command(Cmd),
    Builtin(Builtin),
    Word(Vec<String>),
    Pipe,
    Pipe2,
    And,
    Or,
    Consec,
    Redirect,
    RDAppend,
    Background,
}

#[derive(Debug, Clone)]
pub enum Builtin {
    Cd(Vec<String>),
    Which(Vec<String>),
    Eval(Vec<String>),
    Source(Vec<String>), //use PathBuf instead?
    Echo(Vec<String>),
    Alias(Vec<String>),
    Unalias(Vec<String>),
    Read,
    Kill(Vec<String>),
    Exit,

}

#[derive(Debug, Clone)]
pub struct Cmd {
    pub cmd: String,
    pub args: Vec<String>
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobStatus {
    Completed,
    Stopped,
    Terminated,
}

#[derive(Debug, Clone)]
pub struct Job {
    pub cmds: Vec<Cmd>,
    pub id: i32,
    pub pgid: i32,
    pub status: JobStatus,
}