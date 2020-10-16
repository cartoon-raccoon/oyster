// pub enum Exec {
//     Command(Cmd),
//     Builtin(Builtin),
// }

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    SQuote(String),
    DQuote(String),
    Pipe, //handled!
    Pipe2, //handled!
    And, //handled!
    Or, //handled!
    Consec, //handled!
    FileMarker,
    Redirect,
    RDAppend,
    RDStdOutErr, //Always redirects to a file
    RDFileDesc,  //Redirects to a file descriptor
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Redirect { //* Origin is always a file descriptor
    Override,
    Append,
}

// #[derive(Debug, Clone, PartialEq)]
// pub enum Builtin {
//     Cd(Vec<String>),
//     Which(Vec<String>),
//     Eval(Vec<String>),
//     Source(Vec<String>), //? use PathBuf instead?
//     Export(Vec<String>),
//     Echo(Vec<String>),
//     Alias(Vec<String>),
//     Unalias(Vec<String>),
//     Read,
//     Kill(Vec<String>),
//     Exit,

// }

#[derive(Debug, Clone, PartialEq)]
pub struct Cmd {
    pub cmd: String,
    pub args: Vec<String>,
    pub redirects: Vec<(String, Redirect, String)>,
    pub pipe_stderr: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobStatus {
    InProgress,
    Completed,
    Stopped,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExecCondition {
    And,
    Or,
    Consec,
    Background,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub cmds: Vec<Cmd>,
    pub execnext: Option<ExecCondition>,
    pub id: i32,
    pub pgid: i32,
    pub status: JobStatus,
}