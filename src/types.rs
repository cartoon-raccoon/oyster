use std::fmt;
use std::collections::HashMap;
use std::process;

pub type ParseResult = Result<Vec<Job>, ParseError>;

pub enum TokenizeResult {
    UnmatchedDQuote,
    UnmatchedSQuote,
    EndsOnOr,
    EndsOnAnd,
    EndsOnPipe,
    EmptyCommand,
    Good(Vec<Token>),
}

impl fmt::Display for TokenizeResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenizeResult::UnmatchedDQuote => {
                write!(f, "dquote>")
            }
            TokenizeResult::UnmatchedSQuote => {
                write!(f, "quote>" )
            }
            TokenizeResult::EndsOnAnd => {
                write!(f, "cmdand>")
            }
            TokenizeResult::EndsOnOr => {
                write!(f, "cmdor>" )
            }
            TokenizeResult::EndsOnPipe => {
                write!(f, "pipe>"  )
            }
            TokenizeResult::EmptyCommand => {
                write!(f, "")
            }
            TokenizeResult::Good(_) => {
                write!(f, "")
            }
        }
    }
}

#[derive(Debug)]
pub struct CmdSubError;

impl std::error::Error for CmdSubError {}

impl fmt::Display for CmdSubError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "oyster: parse error in command substitution")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    StartsOnAnd,
    StartsOnOr,
    StartsOnConsec,
    PipeMismatch,
    InvalidFileRD,
    InvalidFileDesc,
    InvalidRDSyntax,
    EmptyCommand,
    Error(String)
}

impl std::error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::StartsOnAnd => {
                write!(f, "error: no command specified before `&&`")
            }
            ParseError::StartsOnOr => {
                write!(f, "error: no command specified before `||`")
            }
            ParseError::StartsOnConsec => {
                write!(f, "error: no command specified before `;`")
            }
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
            ParseError::Error(s) => {
                write!(f, "error: parse error near `{}`", s)
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
    pub capture_stdout: bool,
    pub pipe_stderr: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub cmds: Vec<Cmd>,
    pub execnext: Option<Exec>,
    pub id: i32,
    pub pgid: i32,
    pub status: JobStatus,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JobStatus {
    InProgress,
    Completed,
    Stopped,
    Terminated,
}

#[derive(Debug, Clone)]
pub struct CommandParams {
    pub isatty: bool,
    pub background: bool,
    pub capture_output: bool,
    pub env: HashMap<String, String>,
}

pub struct CommandResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    pub fn new() -> Self {
        CommandResult {
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }
    }
    pub fn from_status(status: i32) -> Self {
        CommandResult {
            status: status,
            stdout: String::new(),
            stderr: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Exec {
    And,
    Or,
    Consec,
    Background,
}

/// A trait to allow for graceful exiting on error instead of panicking.
/// Used to save on match statements for matching results.
pub trait UnwrapOr {
    type Item;

    /// Returns enclosed type if successful and exits with a 
    /// user-specified error code if an error is encountered instead.
    /// 
    /// Does not support displaying the error yet,
    /// only accepts custom error messages.
    fn unwrap_or_exit(self, errmsg: &str, code: i32) -> Self::Item;
}

impl<T,E> UnwrapOr for Result<T,E> {
    type Item = T;

    fn unwrap_or_exit(self, errmsg: &str, code: i32) -> T {
        match self {
            Ok(enclosed) => enclosed,
            Err(_) => {
                eprintln!("{}", errmsg);
                process::exit(code);
            }
        }
    }
}