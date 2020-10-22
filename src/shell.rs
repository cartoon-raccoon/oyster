use std::collections::HashMap;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::fs::OpenOptions;
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
    UnwrapOr,
    Token,
};
use crate::execute;
use crate::parser::Lexer;
use crate::types::{
    CmdSubError,
    TokenizeResult::*
};

#[derive(Clone, Debug)]
pub struct Shell {
    pub jobs: BTreeMap<i32, Job>,
    aliases: HashMap<String, String>,
    pub env: HashMap<String, String>,
    pub vars: HashMap<String, String>,
    current_dir: PathBuf,
    prev_dir: PathBuf,
    pgid: i32,
    pub is_login: bool,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            jobs: BTreeMap::new(),
            aliases: HashMap::new(),
            env: HashMap::new(),
            vars: HashMap::new(),
            current_dir: PathBuf::new(),
            prev_dir: PathBuf::new(),
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
    pub fn add_job(&mut self, job: Job) {
        self.jobs.insert(job.id, job);
    }
    /// Returns the job. Returns None if it doesn't exist.
    pub fn retrieve_job(&mut self, id: i32) -> Option<Job> {
        self.jobs.remove(&id)
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
    pub fn add_variable(&mut self, key: &str, value: &str) {
        self.vars.insert(key.to_string(), value.to_string());
    }
    pub fn get_variable(&self, key: &str) -> Option<String> {
        if let Some(entry) = self.vars.get(key) {
            Some(entry.clone())
        } else {
            None
        }
    }
    /// Loads in a config file and applies it to the shell.
    /// Internally calls the run_script function in execute.
    pub fn with_config() {

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

#[allow(dead_code, unused_variables, unused_mut)]
pub fn search_in_path() {
    let mut path: Vec<String> = env::var("PATH")
        .unwrap_or(String::new())
        .split(":")
        .map(|n| n.to_string())
        .collect();
    if path.is_empty() {
        //return error!
    }

}

//steps:
//expand aliases
//expand tilde
//expand vars
//expand commands

///Only assigns variables if it is the first word in the command.
pub fn assign_variables(shell: &mut Shell, string: &mut String) -> bool {
    let re = Regex::new(r"[a-zA-Z0-9]+=.+").unwrap();
    if re.is_match(string) {
        let key_value: Vec<&str> = string.split("=").collect();
        shell.add_variable(key_value[0], key_value[1]);
        return true;
    }
    false
}

pub fn expand_tilde(string: &mut String) {  
    let mut home = env::var("HOME").unwrap_or(String::new());
    if home.is_empty() {
        eprintln!("oyster: env error, could not expand tilde");
        return;
    }
    home.push_str(string);
    *string = home;
}

//TODO
pub fn expand_braces(string: String) -> Vec<String> {
    Vec::new()
}

//TODO: file globbing, env expansion

pub fn expand_variables(shell: &Shell, string: &mut String) {
    let re = Regex::new(r"\$[a-zA-Z0-9]+").unwrap();
    for capture in re.captures_iter(&string.clone()) {
        if let Some(capture) = capture.get(0) {
            if let Some(var) = shell.get_variable(&capture.as_str()[1..]) {
                *string = string.replacen(capture.as_str(), var.as_str(), 1);
            } else {
                *string = string.replacen(capture.as_str(), "", 1);
            }
        }
    }
}


pub fn replace_aliases(shell: &Shell, word: String) -> String {
    if let Some(string) = shell.get_alias(&word) {
        return string;
    }
    word
}

pub fn needs_substitution(test: &str) -> bool {
    let re_backtick = Regex::new("`[ >&|\\-a-zA-Z0-9\"']+`").unwrap();
    let re_parenths = Regex::new("\\$\\([ >&|\\-a-zA-Z0-9\"']+\\)").unwrap();

    re_backtick.is_match(test) || re_parenths.is_match(test)
}

// This command is gonna be sooo fucking slow
pub fn substitute_commands(shell: &mut Shell, mut string: String) -> Result<String, CmdSubError> {
    let re_backtick = Regex::new("`[ >&|\\-a-zA-Z0-9\"']+`").unwrap();
    //let re_parenths = Regex::new("\\$\\([ >&|\\-a-zA-Z0-9\"']+\\)").unwrap();
    let mut outputs = Vec::<String>::new();
    if let Some(bt_captures) = re_backtick.captures(&string) {
        println!("command matched");
        for capture in bt_captures.iter() {
            if let Some(cmdmatch) = capture {
                let mut newstring = cmdmatch.as_str()[1..].to_string();
                newstring.pop();
                match Lexer::tokenize(shell, newstring, true).unwrap() {
                    UnmatchedDQuote | UnmatchedSQuote => {
                        eprintln!("oyster: unmatched quote");
                        return Err(CmdSubError);
                    }
                    EndsOnAnd | EndsOnOr | EndsOnPipe => {
                        eprintln!("oyster: parse error, ends on delimiter");
                        return Err(CmdSubError);
                    }
                    EmptyCommand => {
                        eprintln!("warning: empty command");
                        return Ok(String::new());
                    }
                    Good(tokens) => {
                        match execute::execute_jobs(shell, tokens, true) {
                            Ok(jobs) => {
                                outputs.push(jobs.1);
                                println!("{:?}", outputs);
                            }
                            Err(e) => {
                                eprintln!("{}", e);
                                return Err(CmdSubError);
                            }
                        }
                    }
                }
            }
        }
        for output in outputs {
            println!("{:?}", output);
            string = re_backtick.replace(
                &string.clone(), 
                output.as_str()
            ).to_string();
        }
    }
    Ok(string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_expand_vars() {
        let mut shell = Shell::new();
        shell.add_variable("hello", "wassup");
        shell.add_variable("what", "is this");
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
            substitute_commands(&mut shell, command).unwrap(),
            String::from("hello")
        )
    }

    #[test]
    fn check_needs_subbing() {
        let command1 = "sudo pacman -Rs `which data`";
        let command2 = "echo which data";
        let command3 = "echo listening to $(cogsy random)";
        assert!(needs_substitution(command1));
        assert!(!needs_substitution(command2));
        assert!(needs_substitution(command3));
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
}