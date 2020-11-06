use std::fmt;
use std::io;
use std::collections::HashMap;
use std::process;

use nix::unistd::Pid;

use crate::shell::{
    Shell,
    expand_variables,
    expand_tilde,
};

pub const STOPPED: i32 = 127;

/// The state of the tokenizer after parsing the current input
pub enum TokenizeResult {
    UnmatchedDQuote,
    UnmatchedSQuote,
    UnmatchedBQuote,
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
            TokenizeResult::UnmatchedBQuote => {
                write!(f, "bquote>")
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

/// The state of the parser after it finishes parsing the given input.
/// Mainly used to detect incomplete scripting constructs.
#[allow(dead_code)] //TODO: Add this in
pub enum ParseResult {
    For(Vec<Job>),
    While(Vec<Job>),
    If,
    Case,
    Good(Vec<Job>),
}

/// A small error type for command substitution to return
#[derive(Debug)]
pub struct CmdSubError;

impl std::error::Error for CmdSubError {}

impl fmt::Display for CmdSubError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "oyster: parse error in command substitution")
    }
}

/// If the Lexer encounters an a pattern that it cannot interpret
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    StartsOnAnd,
    StartsOnOr,
    StartsOnConsec,
    PipeMismatch,
    InvalidFileRD,
    InvalidFileDesc,
    InvalidRDSyntax,
    MetacharsInBrace,
    EmptyCommand,
    GenericError,
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
            ParseError::MetacharsInBrace => {
                write!(f, "error: metacharacters in brace")
            }
            ParseError::EmptyCommand => {
                write!(f, "error: empty command")
            }
            ParseError::GenericError => {
                Ok(())
            }
        }
    }
}

/// A generic error type for all error types to coerce to
#[derive(Debug, Clone)]
pub struct ShellError {
    msg: String,
}

impl std::error::Error for ShellError {}

impl From<ParseError> for ShellError {
    fn from(error: ParseError) -> ShellError {
        ShellError {
            msg: error.to_string()
        }
    }
}

impl From<CmdSubError> for ShellError {
    fn from(error: CmdSubError) -> ShellError {
        ShellError {
            msg: error.to_string()
        }
    }
}

impl From<nix::Error> for ShellError {
    fn from(error: nix::Error) -> ShellError {
        ShellError {
            msg: error.to_string()
        }
    }
}

impl From<io::Error> for ShellError {
    fn from(error: io::Error) -> ShellError {
        ShellError {
            msg: error.to_string()
        }
    }
}

impl From<&str> for ShellError {
    fn from(msg: &str) -> ShellError {
        ShellError {
            msg: String::from(msg)
        }
    }
}

impl From<String> for ShellError {
    fn from(msg: String) -> ShellError {
        ShellError {
            msg: msg
        }
    }
}

// impl<E> From<E> for ShellError where E: Error  {
//     fn from(msg: E) -> ShellError {
//         ShellError {
//             msg: String::from(msg.to_string())
//         }
//     }
// }

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

/// The basic type that shell input is split into
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    SQuote(String),
    DQuote(String),
    BQuote(String),
    Brace(String),
    SqBrkt(String),
    Pipe, //handled!
    Pipe2, //handled!;
    And, //handled!
    Or, //handled!
    Consec, //handled!
    FileMarker,
    Redirect,
    RDAppend,
    RDStdin,
    RDStdOutErr, //Always redirects to a file
    RDFileDesc,  //Redirects to a file descriptor
    Background,
}

/// Produced during parsing, used as a redirect marker in Cmd
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Redirect { //* Origin is always a file descriptor
    Override,
    Append,
    FromStdin,
}

/// Produced during parsing, indicates how the string it comes with
/// should be treated by `Cmd::from_tokencmd()`
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Quote {
    NQuote,
    DQuote,
    SQuote,
    SqBrkt,
}

impl Default for Quote {
    fn default() -> Self {
        Quote::NQuote
    }
}

/// Produced by `parse_tokens()`, it encodes the string type of the text
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TokenCmd {
    pub cmd: (Quote, String),
    pub args: Vec<(Quote, String)>,
    pub redirects: Vec<(String, Redirect, String)>,
    pub pipe_stderr: bool,
}

/// The final data type sent to the core functions
#[derive(Debug, Clone, PartialEq)]
pub struct Cmd {
    pub cmd: String,
    pub args: Vec<String>,
    pub redirects: Vec<(String, Redirect, String)>,
    pub pipe_stderr: bool,
}

impl Cmd {
    /// Checks the quote type and acts on the quote accordingly
    pub fn from_tokencmd(shell: &mut Shell, mut cmd: TokenCmd) -> Self {
        match cmd.cmd.0 {
            Quote::NQuote | Quote::DQuote => {
                expand_variables(shell, &mut cmd.cmd.1);
            }
            Quote::SQuote => {}
            Quote::SqBrkt => {}
        }
        let newargs: Vec<String> = cmd.args.into_iter().map(|(quote, mut string)| {
            match quote {
                Quote::NQuote => {
                    expand_variables(shell, &mut string);
                    expand_tilde(shell, &mut string);
                }
                Quote::DQuote => {
                    expand_variables(shell, &mut string);
                }
                Quote::SQuote => {}
                Quote::SqBrkt => {}
            }
            string
        }).collect();
        Cmd {
            cmd: cmd.cmd.1,
            args: newargs,
            redirects: cmd.redirects,
            pipe_stderr: cmd.pipe_stderr
        }
    }
}

/// Emitted by `parse_tokens()`, its data is consumed by `execute_jobs()`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Job {
    pub cmds: Vec<TokenCmd>,
    pub execnext: Option<Exec>,
    pub id: i32,
}

/// Used by the internal shell HashMap to track jobs in job control
#[derive(Debug, Clone, PartialEq)]
pub struct JobTrack {
    pub firstcmd: String,
    pub id: i32,
    pub pgid: Pid,
    pub pids: Vec<Pid>,
    pub status: JobStatus,
    pub background: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    Running,
    Completed(i32),
    Stopped,
    Signaled(String),
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            JobStatus::Running => {
                write!(f, "In Progress")
            }
            JobStatus::Completed(status) => {
                write!(f, "Done ({})", status)
            }
            JobStatus::Stopped => {
                write!(f, "Stopped")
            }
            JobStatus::Signaled(string) => {
                write!(f, "{}", string)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommandParams {
    pub isatty: bool,
    pub background: bool,
    pub capture_output: bool,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone)]
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

pub enum ExecType {
    Job(Job),
    Script(Vec<Job>),
}

/// A trait to allow for graceful exiting on error instead of panicking.
/// Used to save on match statements for matching results.
pub(crate) trait UnwrapOr {
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

impl<T> UnwrapOr for Option<T> {
    type Item = T;

    fn unwrap_or_exit(self, errmsg: &str, code: i32) -> T {
        match self {
            Some(enclosed) => enclosed,
            None => {
                eprintln!("{}", errmsg);
                process::exit(code);
            }
        }
    }
}