use std::fmt;
use std::io;
use std::collections::HashMap;
use std::process;

use glob::{PatternError, GlobError};

use nix::unistd::Pid;
use nix::sys::signal::Signal;

use crate::shell::{
    Shell,
    eval_sqbrkt,
};
use crate::expansion::{
    expand_variables,
    expand_tilde,
    substitute_commands,
};
use crate::prompt::{
    BOLD,
    RESET,
};

pub const STOPPED: i32 = 127;

/// The state of the tokenizer after parsing the current input
pub enum TokenizeResult {
    UnmatchedDQuote,
    UnmatchedSQuote,
    UnmatchedBQuote,
    UnmatchedCmdSub,
    UnmatchedSqBrkt,
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
                write!(f, "{}dquote > {}", BOLD, RESET)
            }
            TokenizeResult::UnmatchedSQuote => {
                write!(f, "{}quote > {}", BOLD, RESET )
            }
            TokenizeResult::UnmatchedBQuote => {
                write!(f, "{}bquote > {}", BOLD, RESET )
            }
            TokenizeResult::UnmatchedCmdSub => {
                write!(f, "{}cmdsub > {}", BOLD, RESET)
            }
            TokenizeResult::UnmatchedSqBrkt => {
                write!(f, "{}sqbrkt > {}", BOLD, RESET)
            }
            TokenizeResult::EndsOnAnd => {
                write!(f, "{}cmdand > {}", BOLD, RESET )
            }
            TokenizeResult::EndsOnOr => {
                write!(f, "{}cmdor > {}", BOLD, RESET )
            }
            TokenizeResult::EndsOnPipe => {
                write!(f, "{}pipe > {}", BOLD, RESET )
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
#[derive(Debug, Clone)]
pub enum ParseResult {
    For,
    While,
    If,
    Func,
    //Case,
    Good(Vec<Job>),
}

impl fmt::Display for ParseResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseResult::For => {
                write!(f, "{}for > {}", BOLD, RESET )
            }
            ParseResult::While => {
                write!(f, "{}while > {}", BOLD, RESET)
            }
            ParseResult::If => {
                write!(f, "{}if > {}", BOLD, RESET )
            }
            ParseResult::Func => {
                write!(f, "{}func > {}", BOLD, RESET )
            }
            ParseResult::Good(_jobs) => {
                Ok(())
            }
        }
    }
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

impl From<ParseError> for CmdSubError {
    fn from(error: ParseError) -> CmdSubError {
        eprintln!("{}", error);
        CmdSubError
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
    FuncInShellConst,
    InvalidGlob,
    GlobError(String),
    ConversionError,
    GenericError(String),
    EmptyCommand,
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
            ParseError::FuncInShellConst => {
                write!(f, "error: cannot define function in shell construct")
            }
            ParseError::InvalidGlob => {
                write!(f, "error: invalid glob pattern syntax")
            }
            ParseError::GlobError(error) => {
                write!(f, "oyster: {}", error)
            }
            ParseError::ConversionError => {
                write!(f, "oyster: os string conversion error")
            }
            ParseError::GenericError(string) => {
                write!(f, "error: parse error near `{}`", string)
            }
            ParseError::EmptyCommand => {
                write!(f, "error: empty command")
            }
        }
    }
}

impl From<PatternError> for ParseError {
    fn from(_error: PatternError) -> Self {
        ParseError::InvalidGlob
    }
}

impl From<GlobError> for ParseError {
    fn from(error: GlobError) -> Self {
        ParseError::GlobError(error.to_string())
    }
}

/// A generic error type for all error types to coerce to
#[derive(Debug, Clone, PartialEq)]
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

impl From<PatternError> for ShellError {
    fn from(_error: PatternError) -> ShellError {
        ShellError {
            msg: String::from("oyster: invalid glob pattern")
        }
    }
}

impl From<GlobError> for ShellError {
    fn from(error: GlobError) -> ShellError {
        ShellError {
            msg: error.to_string()
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
    CmdSub(String),
    Brace(String),
    SqBrkt(String),
    NmSpce(String),
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

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Token::*;
        match self {
            Word(string) => {
                write!(f, "{}", string)
            }
            SQuote(string) => {
                write!(f, "{}", string)
            }
            DQuote(string) => {
                write!(f, "{}", string)
            }
            BQuote(string) => {
                write!(f, "{}", string)
            }
            CmdSub(string) => {
                write!(f, "{}", string)
            }
            Brace(string) => {
                write!(f, "{}", string)
            }
            NmSpce(string) => {
                write!(f, "${{{}}}", string)
            }
            SqBrkt(string) => {
                write!(f, "{}", string)
            }
            Pipe => {
                write!(f, "|")
            }
            Pipe2 => {
                write!(f, "|&")
            }
            And => {
                write!(f, "&&")
            }
            Or => {
                write!(f, "||")
            }
            Consec => {
                write!(f, ";")
            }
            FileMarker => {
                write!(f, ">&")
            }
            Redirect => {
                write!(f, ">")
            }
            RDAppend => {
                write!(f, ">>")
            }
            RDStdin => {
                write!(f, "<")
            }
            RDStdOutErr => {
                write!(f, "&>")
            }
            RDFileDesc => {
                write!(f, ">&")
            }
            Background => {
                write!(f, "&")
            }
        }
    }
}

/// Produced during parsing, used as a redirect marker in Cmd
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Redirect { //* Origin is always a file descriptor
    Override,
    Append,
    FromStdin,
}

impl Redirect {
    pub fn display(&self) -> String {
        match self {
            Redirect::Override => String::from(">"),
            Redirect::Append => String::from(">>"),
            Redirect::FromStdin => String::from("<")
        }
    }
}

/// Produced during parsing, indicates how the string it comes with
/// should be treated by `Cmd::from_tokencmd()`
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Quote {
    NQuote,
    DQuote,
    SQuote,
    BQuote,
    CmdSub,
    SqBrkt,
    NmSpce,
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

impl fmt::Display for TokenCmd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let cmd: Vec<String> = self.args.iter().map(|(quote, string)| {
            match quote {
                Quote::NQuote => {
                    return format!("{}", string);
                }
                Quote::BQuote => {
                    return format!("{}", string);
                }
                Quote::CmdSub => {
                    return format!("$({})", string);
                }
                Quote::DQuote => {
                    return format!("\"{}\"", string);
                }
                Quote::SQuote => {
                    return format!("'{}'", string);
                }
                Quote::SqBrkt => {
                    return format!("[{}]", string);
                }
                Quote::NmSpce => {
                    return format!("${{{}}}", string);
                }
            }
        }).collect();
        let redirects: Vec<String> = self.redirects.iter().map(
            |(string, rd, string2)| {
                return format!("{}{}{}", string, rd.display(), string2);
            }
        ).collect();
        write!(f, "{} {}", cmd.join(" "), redirects.join(" "))
    }
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
    pub fn from_tokencmd(shell: &mut Shell, mut cmd: TokenCmd) -> Result<Self, ShellError> {
        match cmd.cmd.0 {
            Quote::NQuote => {
                expand_variables(shell, &mut cmd.cmd.1);
                expand_tilde(shell, &mut cmd.cmd.1);
            }
            Quote::DQuote => {
                expand_variables(shell, &mut cmd.cmd.1);
            }
            Quote::CmdSub => {
                match substitute_commands(shell, &cmd.cmd.1) {
                    Ok(string) => {
                        cmd.cmd = (Quote::NQuote, string);
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }
            Quote::BQuote => {
                expand_variables(shell, &mut cmd.cmd.1);
                match substitute_commands(shell, &cmd.cmd.1) {
                    Ok(string) => {
                        cmd.cmd = (Quote::NQuote, string);
                    }
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }
            Quote::NmSpce => {
                //TODO
            }
            Quote::SQuote => {}
            Quote::SqBrkt => {
                cmd.cmd = (Quote::NQuote, eval_sqbrkt(shell, cmd.cmd.1)?.to_string())
            }
        }
        let mut newargs: Vec<String> = Vec::new(); 
        for (quote, mut string) in cmd.args {
            match quote {
                Quote::NQuote => {
                    if string.starts_with("$") {
                        if let Some(var) = shell.get_variable(&string[1..]) {
                            if let Variable::Arr(arr) = var {
                                newargs.extend(arr.into_iter().map(|elem| {
                                    elem.to_string()
                                }).collect::<Vec<String>>());
                                continue;
                            } else {
                                string = var.to_string();
                            }
                        } else {
                            string = String::from("")
                        }
                    }
                    expand_tilde(shell, &mut string);
                }
                Quote::DQuote => {
                    expand_variables(shell, &mut string);
                }
                Quote::CmdSub => {
                    match substitute_commands(shell, &string) {
                        Ok(string) => {
                            let strings: Vec<String> = string.
                            split_whitespace().map(|s| s.to_string())
                            .collect();
                            newargs.extend(strings);
                            continue;
                        }
                        Err(e) => {
                            return Err(e.into());
                        }
                    }
                }
                Quote::BQuote => {
                    expand_variables(shell, &mut string);
                    match substitute_commands(shell, &string) {
                        Ok(string) => {
                            let strings: Vec<String> = string.
                            split_whitespace().map(|s| s.to_string())
                            .collect();
                            newargs.extend(strings);
                            continue;
                        }
                        Err(e) => {
                            return Err(e.into());
                        }
                    }
                }
                Quote::NmSpce => {
                    //TODO
                }
                Quote::SQuote => {}
                Quote::SqBrkt => {
                    newargs.push(eval_sqbrkt(shell, string)?.to_string());
                    continue;
                }
            }
            newargs.push(string);
        }
        Ok(Cmd {
            cmd: cmd.cmd.1,
            args: newargs,
            redirects: cmd.redirects,
            pipe_stderr: cmd.pipe_stderr
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub jobs: Vec<Job>,
    pub params: Option<usize>,
}

impl Function {
    pub fn print(&self) {
        let paramscount = if let Some(count) = self.params {
            count.to_string()
        } else {
            String::from("")
        };
        let jobs: Vec<String> = self.jobs.iter().map(|job| {
            job.to_string()
        }).collect();
        println!(
            "func {} {}\n   {}\nendfn",
            self.name,
            paramscount,
            jobs.join("\n   "),
        )
    }
}

#[derive(Debug, Clone)]
pub struct Map {
    inner: HashMap<String, Variable>,
}

impl Map {

}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Variable {
    Str(String),
    Int(i64),
    Flt(f64),
    Arr(Vec<Variable>),
}

impl Variable {
    pub fn print(&self) {
        match self {
            Variable::Str(string) => {
                println!("str: \"{}\"", string);
            }
            Variable::Int(int) => {
                println!("int: {}", int);
            }
            Variable::Flt(flt) => {
                println!("flt: {}", flt);
            }
            Variable::Arr(arr) => {
                println!("arr: [{}]", arr.iter()
                    .map(|elem| elem.to_string())
                    .collect::<Vec<String>>().join(" ")
                )
            }
        }
    }

    pub fn types_match(lhs: &Variable, rhs: &Variable) -> bool {
        match lhs {
            Variable::Int(_) => {
                if let Variable::Int(_) = rhs {
                    true
                } else {
                    false
                }
            }
            Variable::Flt(_) => {
                if let Variable::Flt(_) = rhs {
                    true
                } else {
                    false
                }
            }
            Variable::Str(_) => {
                if let Variable::Str(_) = rhs {
                    true
                } else {
                    false
                }
            }
            Variable::Arr(_) => {
                if let Variable::Arr(_) = rhs {
                    true
                } else {
                    false
                }
            }
        }
    }
}

impl<V: AsRef<str>> From<V> for Variable {
    fn from(input: V) -> Self {
        let input = input.as_ref();
        if let Ok(int) = input.parse::<i64>() {
            return Variable::Int(int)
        } else if let Ok(flt) = input.parse::<f64>() {
            return Variable::Flt(flt)
        } else {
            Variable::Str(input.to_string())
        }
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Variable::*;
        match self {
            Str(string) => {
                write!(f, "{}", string)
            }
            Int(int) => {
                write!(f, "{}", int)
            }
            Flt(flt) => {
                write!(f, "{}", flt)
            }
            Arr(arr) => {
                write!(f, "{}", arr.iter()
                    .map(|elem| elem.to_string())
                    .collect::<Vec<String>>().join(" ")
                )
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    AddAssgn,
    SubAssgn,
    MulAssgn,
    DivAssgn,
}

/// Emitted by `parse_tokens()`, its data is consumed by `execute_jobs()`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Job {
    pub cmds: Vec<TokenCmd>,
    pub execnext: Option<Exec>,
    pub id: i32,
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let cmds: Vec<String> = self.cmds.iter().map(
            |cmd| {
                return format!("{}", cmd)
            }
        ).collect();
        let exec: String = if let Some(execif) = self.execnext {
            match execif {
                Exec::And => String::from("&&"),
                Exec::Background => String::from("&"),
                Exec::Or => String::from("||"),
                Exec::Consec => String::from("")
            }
        } else {
            String::from("")
        };
        write!(f, "{}{}", cmds.join(" | "), exec)
    }
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
    Signaled(Signal),
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
            JobStatus::Signaled(signal) => {
                write!(f, "{}", signal)
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

impl From<(i32, String)> for CommandResult {
    fn from(from: (i32, String)) -> Self {
        CommandResult {
            status: from.0,
            stdout: from.1,
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