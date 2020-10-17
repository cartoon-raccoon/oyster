use std::fmt;

pub type ParseResult = Result<Vec<Job>, ParseError>;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ParseError {
    PipeMismatch,
    InvalidFileRD,
    InvalidFileDesc,
    InvalidRDSyntax,
    EmptyCommand,
}

impl std::error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::PipeMismatch => {
                write!(f, "error: pipe mismatch")
            },
            ParseError::InvalidFileRD => {
                write!(f, "error: redirecting to invalid file")
            },
            ParseError::InvalidFileDesc => {
                write!(f, "error: redirecting to invalid file descriptor")
            },
            ParseError::InvalidRDSyntax => {
                write!(f, "error: invalid redirection syntax")
            },
            ParseError::EmptyCommand => {
                write!(f, "error: empty command")
            }
        }
    }
}

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