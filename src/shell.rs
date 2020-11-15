use std::collections::{HashMap, BTreeMap};
use std::path::PathBuf;
use std::fs::{OpenOptions, File};
use std::os::unix::io::IntoRawFd;
use std::env;

use regex::Regex;

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
    Map,
    Operator,
    Function,
    UnwrapOr,
    JobStatus,
    ShellError,
};
use crate::execute;
use crate::scripting::execute_scriptfile;

#[derive(Clone, Debug)]
pub struct Shell {
    pub jobs: BTreeMap<i32, JobTrack>,
    aliases: HashMap<String, String>,
    pub env: HashMap<String, String>,
    vars: HashMap<String, Var>,
    maps: HashMap<String, Map>,
    pub funcs: HashMap<String, Function>,
    max_nesting: usize,
    stack_size: usize,
    pub dirstack: Vec<PathBuf>,
    pub current_dir: PathBuf,
    pub prev_dir: PathBuf,
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
            maps: HashMap::new(),
            funcs: HashMap::new(),
            max_nesting: 50,
            stack_size: 0,
            dirstack: Vec::new(),
            current_dir: PathBuf::from(pwd),
            prev_dir: PathBuf::from(home),
            pgid: 0,
            is_login: false,
        }
    }
    pub fn change_dir<P: Into<PathBuf>>(&mut self, cd_to: P) -> Result<(), ShellError> {
        let cd_to = cd_to.into();
        self.prev_dir = self.current_dir.clone();
        self.current_dir = PathBuf::from(&cd_to);
        match env::set_current_dir(&cd_to) {
            Ok(()) => {
                env::set_var("PWD", &cd_to);
                return Ok(());
            }
            Err(e) => {
                if e.to_string().contains("No such file or directory") {
                    return Err(ShellError::from("no such file or directory"))
                }
                return Err(ShellError::from(e.to_string()));
            }
        }
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
            if self.stack_size == self.max_nesting {
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
            if self.stack_size > 0{
                self.stack_size -= 1;
            }
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

pub fn eval_sqbrkt(shell: &mut Shell, string: String)
-> Result<Var, ShellError> {
    let string_error: &'static str = 
    "oyster: operators other than `+` are not supported for strings";
    let (lhs, op, rhs) = match tokenize_sqbrkt(shell, &string) {
        Ok(ops) => ops,
        Err(_) => return Ok(Var::Str(string))
    };
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
            Var::Arr(_) => {
                Err(ShellError::from("oyster: cannot operate on arrays"))
            }
        }
    } else {
        Err(ShellError::from("oyster: mismatched variable types (operator evaluation)"))
    }
}

//Note: I wanted to do generics here but the kind of trait bounds
//required to use for the items I wanted to return are not supported by Rust.
fn perform_ops_int(lhs: i64, op: Operator, rhs: i64)
-> Result<i64, ShellError> {
    use Operator::*;
    let unsupported_err: &'static str = 
    "oyster: assignment operators are currently unsupported";
    match op {
        Add => {
            if lhs == i64::MAX {
                return Err(ShellError::from("error: variable overflow"))
            }
            Ok(lhs + rhs)
        }
        Sub => {
            if lhs == i64::MIN {
                return Err(ShellError::from("error: variable underflow"))
            }
            Ok(lhs - rhs)
        }
        Mul => {
            if lhs as i128 * rhs as i128 >= i64::MAX as i128 {
                return Err(ShellError::from("error: variable overflow"))
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

fn perform_ops_flt(lhs: f64, op: Operator, rhs: f64)
-> Result<f64, ShellError> {
    use Operator::*;
    let unsupported_err: &'static str = 
    "oyster: assignment operators are currently unsupported";
    match op {
        Add => {
            if lhs == f64::MAX {
                return Err(ShellError::from("error: variable overflow"))
            }
            Ok(lhs + rhs)
        }
        Sub => {
            if lhs == f64::MIN {
                return Err(ShellError::from("error: variable underflow"))
            }
            Ok(lhs - rhs)
        }
        Mul => {
            if lhs as i128 * rhs as i128 >= f64::MAX as i128 {
                return Err(ShellError::from("error: variable overflow"))
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

fn tokenize_sqbrkt(shell: &mut Shell, string: &str)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

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
}