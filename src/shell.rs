use std::collections::{HashMap, BTreeMap};
use std::path::PathBuf;
use std::fs::{OpenOptions, File};
use std::os::unix::io::IntoRawFd;
use std::env;

use regex::Regex;

use glob::glob;

use nix::unistd::{
    Pid,
    tcsetpgrp,
};
use nix::sys::signal::{
    Signal,
    SigSet,
    pthread_sigmask,
    SigmaskHow,
};

use crate::types::{
    Job,
    JobTrack,
    Variable as Var,
    Operator,
    Function,
    UnwrapOr,
    JobStatus,
    ShellError,
    ParseError,
    Quote,
};
use crate::execute;
use crate::parser::Lexer;
use crate::types::{
    CmdSubError,
    TokenizeResult::*,
    ParseResult,
};
use crate::scripting::execute_scriptfile;

#[derive(Clone, Debug)]
pub struct Shell {
    pub jobs: BTreeMap<i32, JobTrack>,
    aliases: HashMap<String, String>,
    pub env: HashMap<String, String>,
    pub vars: HashMap<String, Var>,
    pub funcs: HashMap<String, Function>,
    pub max_recursion: usize,
    pub stack_size: usize,
    current_dir: PathBuf,
    prev_dir: PathBuf,
    pgid: i32,
    pub is_login: bool,
}

impl Shell {
    pub fn new() -> Self {
        let home = env::var("HOME").unwrap_or(String::new());
        let pwd = env::var("PWD").unwrap_or(String::new());
        Shell {
            jobs: BTreeMap::new(),
            aliases: HashMap::new(),
            env: HashMap::new(),
            vars: HashMap::new(),
            funcs: HashMap::new(),
            max_recursion: 50,
            stack_size: 0,
            current_dir: PathBuf::from(pwd),
            prev_dir: PathBuf::from(home),
            pgid: 0,
            is_login: false,
        }
    }
    pub fn set_prev_dir(&mut self, path: String) {
        self.prev_dir = PathBuf::from(path);
    }
    pub fn set_current_dir(&mut self, path: String) {
        self.current_dir = PathBuf::from(path);
    }
    /// Adds a job to the shell to track.
    /// Normally only used for background jobs.
    pub fn add_cmd_to_job(
        &mut self, 
        id: i32, 
        pid: Pid, 
        pgid: Pid, 
        cmd: String,
        bg: bool,
    ) {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.pids.push(pid);
            return;
        } else {
            self.jobs.insert(id, JobTrack {
                firstcmd: cmd,
                id: id,
                pgid: pgid,
                pids: vec![pid],
                status: JobStatus::Running,
                background: bg,
            });
        }
    }
    /// Returns the job. Returns None if it doesn't exist.
    pub fn remove_pid_from_job(&mut self, pid: Pid, pgid: Pid) 
    -> Option<JobTrack> {
        let mut pids_empty = false;
        let mut jobid = 0;
        for (id, job) in self.jobs.iter_mut() {
            if pgid == job.pgid {
                if let Ok(idx) = job.pids.binary_search(&pid) {
                    job.pids.remove(idx);
                }
                pids_empty = job.pids.is_empty();
                jobid = *id;
            }
        }
        if pids_empty {
            return self.jobs.remove(&jobid);
        }
        None
    }

    pub fn get_job_by_pgid(&mut self, pgid: Pid) -> Option<&JobTrack> {
        for job in &self.jobs {
            if job.1.pgid == pgid {
                return Some(&job.1)
            }
        }
        None
    }

    pub fn get_job_by_id(&mut self, id: i32) -> Option<&JobTrack> {
        self.jobs.get(&id)
    }

    pub fn mark_job_as_stopped(&mut self, id: i32) {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.status = JobStatus::Stopped;
        }
    }

    pub fn mark_job_as_running(&mut self, id: i32, bg: bool) {
        if let Some(job) = self.jobs.get_mut(&id) {
            job.status = JobStatus::Running;
            if bg {
                job.background = true;
            }
        }
    }
    pub fn insert_func(&mut self, name: &str, jobs: Vec<Job>, params: Option<usize>) {
        let func = Function {
            name: name.to_string(),
            jobs: jobs,
            params: params,
        };
        self.funcs.insert(name.to_string(), func);
    }
    pub fn execute_func(&mut self, name: &str, params: Vec<String>) 
    -> Result<(i32, String), ShellError> {
        if let Some(func) = &mut self.funcs.get(name) {
            if self.stack_size == self.max_recursion {
                self.stack_size = 0;
                return Err(ShellError::from("oyster: exceeded maximum recursion depth"))
            }
            self.stack_size += 1;
            let jobs_to_do = func.jobs.clone();
            if let Some(paramscount) = func.params {
                if paramscount != params.len() {
                    return Err(
                        ShellError::from("oyster: function parameter mismatch")
                    )
                }
            }
            let mut counter = 0;
            for param in params {
                let varname = format!("{}{}", name, counter);
                self.add_variable(&varname, Var::from(param));
                counter += 1;
            }
            let result = execute::execute_jobs(self, jobs_to_do, false);
            for i in 0..counter {
                let varname = format!("{}{}", name, i);
                self.remove_variable(&varname);
            }
            result
        } else {
            let msg = format!("oyster: no function `{}` found", name);
            return Err(ShellError::from(msg))
        }
    }
    /// Called by the alias builtin.
    /// Adds an alias to the shell.
    pub fn add_alias(&mut self, key: &str, value: &str) {
        self.aliases.insert(key.to_string(), value.to_string());
    }
    /// Called by the unalias builtin.
    /// Removes an alias from the shell.
    pub fn remove_alias(&mut self, key: &str) -> Option<String> {
        self.aliases.remove(key)
    }
    /// Tests whether the alias exists.
    pub fn has_alias(&self, key: &str) -> bool {
        self.aliases.contains_key(key)
    }
    /// Returns the value of an alias if it exists in the shell.
    /// Normally called internally during alias replacement
    /// and should not be invoked manually by the user.
    pub fn get_alias(&self, key: &str) -> Option<String> {
        if let Some(entry) = self.aliases.get(key) {
            Some(entry.clone())
        } else {
            None
        }
    }
    /// Adds a variable to the shell.
    pub fn add_variable(&mut self, key: &str, value: Var) {
        self.vars.insert(key.to_string(), value);
    }
    /// Gets the value of a variable from the shell without removing it.
    pub fn get_variable(&self, key: &str) -> Option<Var> {
        if let Some(entry) = self.vars.get(key) {
            Some(entry.clone())
        } else {
            None
        }
    }
    /// Removes a variable from the shell.
    pub fn remove_variable(&mut self, key: &str) -> Option<String> {
        self.vars.remove(key).map(|var| {
            var.to_string()
        })
    }
    /// Loads in a config file and applies it to the shell.
    /// Internally calls the run_script function in execute.
    pub fn with_config(filename: &str) -> Self {
        let mut shell = Shell::new();
        //todo FIXME: variables not registering with shell
        for (var, value) in env::vars() {
            shell.add_variable(&var, Var::Str(value));
        }
        match execute_scriptfile(&mut shell, filename) {
            Ok(status) => {
                if status != 0 {
                    eprintln!("oyster: error occurred while running rcfile");
                }
            },
            Err(e) => {
                eprintln!("{}", e);
                eprintln!("oyster: cannot start shell with config");
            }
        }
        shell
    }
}

pub fn give_terminal_to(pgid: Pid) -> nix::Result<bool> {
    let mut mask = SigSet::empty();
    let mut old_mask = SigSet::empty();

    mask.add(Signal::SIGTTIN);
    mask.add(Signal::SIGTTOU);
    mask.add(Signal::SIGTSTP);
    mask.add(Signal::SIGCHLD);

    pthread_sigmask(SigmaskHow::SIG_BLOCK, 
                    Some(&mask), 
                    Some(&mut old_mask)
                )?;
    tcsetpgrp(1, pgid)?;
    pthread_sigmask(SigmaskHow::SIG_SETMASK, 
                    Some(&old_mask), 
                    Some(&mut mask)
                )?;
    Ok(true)
}

pub fn create_fd_from_file(dest: &str, to_append: bool) -> i32 {
    let mut file = OpenOptions::new();
    if to_append {
        file.append(true);
    } else {
        file.write(true).truncate(true);
    }
    let file = file.create(true).open(dest)
        .unwrap_or_exit("oyster: could not create file", 3);
    file.into_raw_fd()
}

pub fn open_file_as_fd(dest: &str) -> i32 {
    File::open(dest)
    .unwrap_or_exit("oyster: could not open file", 3)
    .into_raw_fd()
}

pub fn search_in_path(command: &str) -> Result<PathBuf, ShellError> {
    //collecting all entries in $PATH
    let paths: Vec<PathBuf> = env::var("PATH")
        .unwrap_or(String::new())
        .split(":")
        .map(|n| PathBuf::from(n))
        .collect();
    if paths.is_empty() {
        return Err(ShellError::from("oyster: path is empty"))
    }
    for path in paths {
        //iterating over all the entries in the path
        for item in std::fs::read_dir(path)? {
            let item = item?;
            //getting the file name of the entry path
            if let Some(entry) = item.path().file_name() {
                let entry = entry.to_str()
                    .ok_or(ShellError::from("oyster: error converting filepaths"))?;
                if entry == command {
                    return Ok(item.path())
                } else {
                    continue
                }
            } else {
                return Err(ShellError::from(
                    format!("oyster: error")
                ))
            }
        }
    }
    Err(ShellError::from(format!("oyster: command `{}` not found", command)))
}

//steps:
//expand aliases
//expand tilde
//expand vars
//expand commands

///Only assigns variables if it is the first word in the command.
pub fn assign_variables(shell: &mut Shell, string: &mut String) -> bool {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[a-zA-Z0-9_]+=.+").unwrap();
    }
    if RE.is_match(string) {
        let key_value: Vec<&str> = string.split("=").collect();
        shell.add_variable(key_value[0], Var::from(key_value[1]));
        return true;
    }
    false
}

pub fn expand_tilde(shell: &mut Shell, string: &mut String) {  
    let home = env::var("HOME").unwrap_or(String::new());
    if home.is_empty() {
        eprintln!("oyster: env error, could not expand tilde");
        return;
    }
    if string.starts_with("~") {
        if string.starts_with("~+") {
            let pwd = shell.current_dir.to_str().unwrap_or("");
            *string = string.replacen("~+", pwd, 1);
        } else if string.starts_with("~-") {
            let oldpwd = shell.prev_dir.to_str().unwrap_or("");
            *string = string.replacen("~-", oldpwd, 1);
        } else {
            *string = string.replacen("~", &home, 1);
        }
    } else {
        return;
    }
}

//* I stole this from https://rosettacode.org/wiki/Brace_expansion#Rust
//* I hate that I couldn't figure it out and had to steal.
//* I promise I'll implemente this by hand one day.
mod brace_expansion {
    const OPEN_CHAR: char = '{';
    const CLOSE_CHAR: char = '}';
    const SEPARATOR: char = ',';
    const ESCAPE: char = '\\';
    
    #[derive(Debug, PartialEq, Clone)]
    pub enum Token {
        Open,
        Close,
        Separator,
        Payload(String),
        Branches(Branches),
    }
    
    impl From<char> for Token {
        fn from(ch: char) -> Token {
            match ch {
                OPEN_CHAR => Token::Open,
                CLOSE_CHAR => Token::Close,
                SEPARATOR => Token::Separator,
                _ => panic!("Non tokenizable char!"),
            }
        }
    }
    
    #[derive(Debug, PartialEq, Clone)]
    pub struct Branches {
        tokens: Vec<Vec<Token>>,
    }
    
    impl Branches {
        fn new() -> Branches {
            Branches{
                tokens: Vec::new(),
            }
        }
    
        fn add_branch(&mut self, branch: Vec<Token>) {
            self.tokens.push(branch);
        }
    
        fn from(tokens: &Vec<Token>) -> Branches {
            let mut branches = Branches::new();
            let mut tail = tokens.clone();
            while let Some(pos) = tail.iter().position(|token| { *token == Token::Separator }) {
                let mut rest = tail.split_off(pos);
                branches.add_branch(tail);
                rest.remove(0);
                tail = rest;
            }
            branches.add_branch(tail);
            branches
        }
    }
    
    impl From<Branches> for Token {
        fn from(branches: Branches) -> Token {
            Token::Branches(branches)
        }
    }
    
    impl From<Vec<Token>> for Branches {
        fn from(tokens: Vec<Token>) -> Branches {
            Branches::from(&tokens)
        }
    }
    
    impl From<Token> for String {
        fn from(token: Token) -> String {
            match token {
                Token::Branches(_) => panic!("Cannot convert to String!"),
                Token::Payload(text) => text,
                Token::Open => OPEN_CHAR.to_string(),
                Token::Close => CLOSE_CHAR.to_string(),
                Token::Separator => SEPARATOR.to_string(),
            }
        }
    }
    
    impl From<Branches> for Vec<String> {
        fn from(branches: Branches) -> Vec<String> {
            let Branches{ tokens: token_lines } = branches;
            let mut vec: Vec<String> = Vec::new();
            let braces = { if token_lines.len() == 1 { true } else { false } };
            for tokens in token_lines {
                let mut vec_string = output(tokens);
                vec.append(&mut vec_string);
            }
            if braces {
                vec.iter()
                    .map(|line| {
                        format!("{}{}{}", OPEN_CHAR, line, CLOSE_CHAR)
                    }).
                    collect::<Vec<String>>()
            } else {
                vec
            }
        }
    }
    
    impl From<Token> for Vec<String> {
        fn from(token: Token) -> Vec<String> {
            match token {
                Token::Branches(branches) => {
                    branches.into()
                },
                _ => {
                    let frag: String = token.into();
                    vec![frag]
                },
            }
        }
    }
    
    pub fn tokenize(string: &str) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();
        let mut chars = string.chars();
        let mut payload = String::new();
        while let Some(ch) = chars.next() {
            match ch {
                OPEN_CHAR | SEPARATOR | CLOSE_CHAR => {
                    if payload.len() > 0 {
                        tokens.push(Token::Payload(payload));
                    }
                    payload = String::new();
                    if ch == CLOSE_CHAR {
                        let pos = tokens.iter().rposition(|token| *token == Token::Open);
                        if let Some(pos) = pos {
                            let branches: Branches = {
                                let mut to_branches = tokens.split_off(pos);
                                to_branches.remove(0);
                                to_branches
                            }.into();
                            tokens.push(branches.into());
                        } else {
                            tokens.push(ch.into());
                        }
                    } else {
                        tokens.push(ch.into());
                    }
                },
                ESCAPE => {
                    payload.push(ch);
                    if let Some(next_char) = chars.next() {
                        payload.push(next_char);
                    }
                },
                _ => payload.push(ch),
            }
        }
        let payload = payload.trim_end();
        if payload.len() > 0 {
            tokens.push(Token::Payload(payload.into()));
        }
        tokens
    }
    
    pub fn output(tokens: Vec<Token>) -> Vec<String> {
        let mut output: Vec<String> = vec![String::new()];
        for token in tokens {
            let mut aux: Vec<String> = Vec::new();
            let strings: Vec<String> = token.into();
            for root in &output {
                for string in &strings {
                    aux.push(format!("{}{}", root, string));
                }
            }
            output = aux;
        }
        output
    }
}

//TODO
pub fn expand_braces(string: String) 
-> Vec<(Quote, String)> {
    let output = brace_expansion::tokenize(&string);
    brace_expansion::output(output).into_iter()
    .map(|string| (Quote::NQuote, string)).collect()
}

//TODO: file globbing, env expansion

pub fn expand_variables(shell: &Shell, string: &mut String) {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"\$[a-zA-Z0-9_]+").unwrap();
    }
    for capture in RE.captures_iter(&string.clone()) {
        if let Some(capture) = capture.get(0) {
            if let Some(var) = shell.get_variable(&capture.as_str()[1..]) {
                *string = string.replacen(capture.as_str(), &var.to_string(), 1);
            } else {
                *string = string.replacen(capture.as_str(), "", 1);
            }
        }
    }
}

pub fn expand_glob(string: &str) -> Result<Vec<String>, ParseError> {
    let mut to_return = Vec::new();
    for path in glob(string)? {
        let path = path?;
        to_return.push(path.to_str().ok_or(
            ParseError::ConversionError
        )?.to_string());
    }
    Ok(to_return)
}

pub fn eval_sqbrkt(shell: &mut Shell, string: String)
-> Result<Var, ShellError> {
    let string_error: &'static str = 
    "oyster: operators other than `+` are not supported for strings";
    let (lhs, op, rhs) = tokenize_sqbrkt(shell, string)?;
    if Var::types_match(&lhs, &rhs) {
        match lhs {
            Var::Str(string) => {
                if let Var::Str(string2) = rhs {
                    if let Operator::Add = op {
                        Ok(Var::Str(string + &string2))
                    } else {
                        Err(ShellError::from(string_error))
                    }
                } else {
                    unreachable!()
                }
            }
            Var::Int(number) => {
                if let Var::Int(number2) = rhs {
                    Ok(Var::Int(perform_ops_int(number, op, number2)?))
                } else {
                    unreachable!()
                }
            }
            Var::Flt(float)  => {
                if let Var::Flt(float2) = rhs {
                    Ok(Var::Flt(perform_ops_flt(float, op, float2)?))
                } else {
                    unreachable!()
                }
            }
        }
    } else {
        Err(ShellError::from("oyster: mismatched variable types"))
    }
}

//Note: I wanted to do generics here but the kind of trait bounds
//required to use for the items I wanted to return are not supported by Rust.
fn perform_ops_int(lhs: i32, op: Operator, rhs: i32)
-> Result<i32, ShellError> {
    use Operator::*;
    let unsupported_err: &'static str = 
    "oyster: assignment operators are currently unsupported";
    match op {
        Add => {
            if lhs == i32::MAX {
                return Err(ShellError::from("error: variable overflow"))
            }
            Ok(lhs + rhs)
        }
        Sub => {
            if lhs == i32::MIN {
                return Err(ShellError::from("error: variable underflow"))
            }
            Ok(lhs - rhs)
        }
        Mul => {
            if lhs as i64 * rhs as i64 >= i32::MAX as i64 {
                return Err(ShellError::from("error variable overflow"))
            }
            Ok(lhs * rhs)
        }
        Div => {
            Ok(lhs / rhs)
        }
        _ => {
            Err(ShellError::from(unsupported_err))
        }
    }
}

fn perform_ops_flt(lhs: f32, op: Operator, rhs: f32)
-> Result<f32, ShellError> {
    use Operator::*;
    let unsupported_err: &'static str = 
    "oyster: assignment operators are currently unsupported";
    match op {
        Add => {
            Ok(lhs + rhs)
        }
        Sub => {
            Ok(lhs - rhs)
        }
        Mul => {
            Ok(lhs * rhs)
        }
        Div => {
            Ok(lhs / rhs)
        }
        _ => {
            Err(ShellError::from(unsupported_err))
        }
    }
}

fn tokenize_sqbrkt(shell: &mut Shell, string: String)
-> Result<(Var, Operator, Var), ShellError> {
    let mut in_quote = false;
    let mut parsed: Vec<(bool, String)> = Vec::new();
    let mut word = String::new();
    for c in string.chars() {
        match c {
            '"' => {
                if !in_quote {
                    in_quote = true;
                } else {
                    parsed.push((true, word.clone()));
                    word.clear();
                    in_quote = false;
                }
            }
            ' ' => if !in_quote {
                if !word.is_empty() {
                    parsed.push((false, word.clone()));
                }
                word.clear();
            }
            _ => {
                word.push(c);
            }
        }
    }
    if !word.is_empty() {
        parsed.push((false, word));
    }
    if parsed.len() != 3 {
        return Err(
            ShellError::from(
                "oyster: cannot parse square bracket"
            )
        )
    }
    if parsed[1].0 {
        return Err(
            ShellError::from(
                "oyster: middle operator is quoted"
            )
        )
    }
    let lhs = if parsed[0].0 {
        Var::Str(parsed[0].1.clone())
    } else if parsed[0].1.starts_with("$") {
        if let Some(var) = shell.get_variable(&parsed[0].1[1..]) {
            var
        } else {
            return Err(
                ShellError::from(
                    format!("oyster: variable {} not found", &parsed[0].1[1..])
                )
            )
        }
    } else {
        Var::from(&parsed[0].1)
    };
    let rhs = if parsed[2].0 {
        Var::Str(parsed[2].1.clone())
    } else if parsed[2].1.starts_with("$") {
        if let Some(var) = shell.get_variable(&parsed[2].1[1..]) {
            var
        } else {
            return Err(
                ShellError::from(
                    format!("oyster: variable {} not found", &parsed[2].1[1..])
                )
            )
        }
    } else {
        Var::from(&parsed[2].1)
    };
    let op = match parsed[1].1.as_str() {
        "+"  => {Operator::Add}
        "-"  => {Operator::Sub}
        "*"  => {Operator::Mul}
        "/"  => {Operator::Div}
        "+=" => {Operator::AddAssgn}
        "-=" => {Operator::SubAssgn}
        "*=" => {Operator::MulAssgn}
        "/=" => {Operator::DivAssgn}
        n@ _ => {
            return Err(ShellError::from(
                format!("oyster: invalid operator {}", n)
            ))
        }
    };
    Ok((lhs, op, rhs))
}

pub fn replace_aliases(shell: &Shell, word: String) -> String {
    if let Some(string) = shell.get_alias(&word) {
        return string;
    }
    word
}

// This command is gonna be sooo fucking slow
pub fn substitute_commands(shell: &mut Shell, string: &str) -> Result<String, CmdSubError> {
    let mut string = string.to_string();
    // Tokenizing and capturing cmbsubs first
    lazy_static! {
        static ref CMDSUB_RE: Regex = Regex::new(
            "\\$\\([\\a-zA-Z0-9 \"-.@~/\\|<>\\&$()]+\\)"
        ).unwrap();
    }
    for capture in CMDSUB_RE.captures_iter(&string.clone()) {
        if let Some(capture) = capture.get(0) {
            let mut capture = capture.as_str().to_string();
            capture.pop();
            let output = execute_commands_once(shell, &capture[2..])?;
            capture.push(')');
            string = string.replacen(&capture, &output, 1);
        }
    }
    let mut stringchars = string.chars();
    let mut captures: Vec<String> = Vec::new();
    let mut rest: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut in_quote = false;
    while let Some(c) = stringchars.next() {
        match c {
            '`' if !in_quote => {
                in_quote = true;
                rest.push(word.clone());
                word.clear();
            }
            '`' if in_quote => {
                in_quote = false;
                captures.push(word.clone());
                word.clear();
            }
            '\\' => {
                if let Some(c) = stringchars.next() {
                    word.push(c);
                    continue;
                }
            }
            _ => {
                word.push(c);
            }
        }
    }
    rest.push(word);
    let mut outputs = Vec::<String>::new();
    for capture in captures {
        outputs.push(execute_commands_once(shell, &capture)?);
    }
    let mut final_str = String::new();
    let mut outputs = outputs.iter();
    for string in rest {
        final_str.push_str(&string);
        if let Some(output) = outputs.next() {
            final_str.push_str(output)
        }
    }

    Ok(final_str)
}

fn execute_commands_once(shell: &mut Shell, input: &str) 
-> Result<String, CmdSubError> {
    if let Ok(result) = Lexer::tokenize(input) {
        match result {
            UnmatchedDQuote | UnmatchedSQuote | UnmatchedBQuote => {
                eprintln!("error: unmatched quote");
                return Err(CmdSubError);
            }
            UnmatchedCmdSub => {
                eprintln!("error: unmatched command substitution");
                return Err(CmdSubError);
            }
            UnmatchedSqBrkt => {
                eprintln!("error: unmatched square bracket");
                return Err(CmdSubError);
            }
            EndsOnAnd | EndsOnOr | EndsOnPipe => {
                eprintln!("error: command ends on delimiter");
                return Err(CmdSubError);
            }
            EmptyCommand => {
                eprintln!("warning: empty command");
                return Ok(String::new());
            }
            Good(tokens) => {
                // expand_variables(shell, &mut tokens);
                if let ParseResult::Good(jobs) = Lexer::parse_tokens(shell, tokens)? {
                    match execute::execute_jobs(shell, jobs, true) {
                        Ok(mut jobs) => {
                            if let Some('\n') = jobs.1.chars().last() {
                                jobs.1.pop();
                            }
                            Ok(jobs.1)
                        }
                        Err(e) => {
                            eprintln!("error while executing: {}", e);
                            return Err(CmdSubError);
                        }
                    }
                } else {
                    eprintln!("error: incomplete shell struct");
                    return Err(CmdSubError);
                }
            }
        }
    } else {
        eprintln!("error: tokenization error");
        Err(CmdSubError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn check_expand_vars() {
        let mut shell = Shell::new();
        shell.add_variable("hello", Var::from("wassup"));
        shell.add_variable("what", Var::from("is this"));
        let mut test = String::from("goodbye $hello i know you $what $wontwork");
        expand_variables(&shell, &mut test);
        assert_eq!(
            test,
            String::from("goodbye wassup i know you is this ")
        );
    }

    #[test]
    fn check_alias_replacement() {
        let mut shell = Shell::new();
        shell.add_alias(
            "addpkg",
            "sudo pacman -S"
        );
        shell.add_alias(
            "yeet",
            "sudo pacman -Rs",
        );
        let test_string = String::from("addpkg");
        let new_string = replace_aliases(&shell, test_string);
        assert_eq!(
            new_string,
            String::from("sudo pacman -S"),
        );
        let test_string2 = String::from("yeet");
        let new_string2 = replace_aliases(&shell, test_string2);
        assert_eq!(
            new_string2,
            String::from("sudo pacman -Rs")
        );
    }

    #[test]
    fn check_command_substitution() { //* This test fails
        let mut shell = Shell::new();
        let command = String::from("`echo hello`");
        assert_eq!(
            substitute_commands(&mut shell, &command).unwrap(),
            String::from("hello")
        )
    }

    #[test]
    fn check_variable_assignment() {
        let mut shell = Shell::new();
        let mut string = String::from("wassup=hello");
        let mut string2 = String::from("what=is this");
        let mut fail = String::from("hello i am stupid");
        assert!(assign_variables(
            &mut shell,
            &mut string,
        ));
        assert!(assign_variables(
            &mut shell, 
            &mut string2,
        ));
        assert!(!assign_variables(
            &mut shell,
            &mut fail,
        ));
    }
    #[test]
    fn check_path_searching() {
        let command = OsString::from("cogsy");
        assert_eq!(
            OsString::from("/home/sammy/.cargo/bin/cogsy"), 
            search_in_path(command.to_str()
            .unwrap()).unwrap()
        );
        let command = OsString::from("pacman");
        assert_eq!(
            OsString::from("/usr/bin/pacman"),
            search_in_path(command.to_str()
            .unwrap()).unwrap()
        )
    }

    #[test]
    fn check_path_globbing() { 
        //this fails because i couldn't be bothered to type out everything
        //but the function works correctly
        let globres = expand_glob("/home/sammy/Projects/oyster/*").unwrap();
        assert_eq!(globres, vec![
            String::from("/home/sammy/Projects/oyster/src"),
            String::from("/home/sammy/Projects/oyster/target"),
            String::from("/home/sammy/Projects/oyster/.gitignore"),
        ])
    }
}