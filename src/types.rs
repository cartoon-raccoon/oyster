#[derive(Debug, Clone)]
pub enum Token {
    Command(Cmd),
    Builtin(Builtin),
    Word(Vec<String>),
    Pipe,
    Pipe2,
    And,
    Or,
    Redirect,
    RDAppend,
    Background,
}

#[derive(Debug, Clone)]
pub enum Builtin {
    Cd(Vec<String>),
    Which(Vec<String>),
    Eval(String),
    Source(String), //use PathBuf instead?
    Echo(String),
    Alias(String),
    Unalias(String),
    Read,
    Kill(Vec<String>),
    Exit,

}

#[derive(Debug, Clone)]
pub struct Cmd {
    pub cmd: String,
    pub args: Vec<String>
}

pub struct Job {
    cmd: Cmd,
}