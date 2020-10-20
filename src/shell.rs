use std::collections::HashMap;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::os::unix::io::IntoRawFd;

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
};
use crate::execute;
use crate::parser::Lexer;
use crate::types::{
    CmdSubError,
    ParseError,
    TokenizeResult::*
};

#[derive(Clone, Debug)]
pub struct Shell {
    jobs: BTreeMap<i32, Job>,
    aliases: HashMap<String, String>,
    pub env: HashMap<String, String>,
    vars: HashMap<String, String>,
    current_dir: PathBuf,
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
            pgid: 0,
            is_login: false,
        }
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
    pub fn add_alias(&mut self, key: String, value: String) {
        self.aliases.insert(key, value);
    }
    /// Called by the unalias builtin.
    /// Removes an alias from the shell.
    pub fn remove_alias(&mut self, key: &str) {
        self.aliases.remove(key);
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
    pub fn add_variable(&mut self, key: String, value: String) {
        self.vars.insert(key, value);
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

//steps:
//expand aliases
//expand tilde
//expand vars
//expand commands

//TODO: Command expansion, file globbing, tilde and env expansion
pub fn expand_variables(shell: &Shell, tokens: &mut Vec<String>) {
    let re = Regex::new(r"\$[a-zA-Z]*").unwrap();
    for token in tokens {
        if re.is_match(&token) {
            if let Some(string) = shell.get_variable(&token) {
                *token = string;
            } else {
                *token = String::from("");
            }
        }
    }
}


//TODO: Find a way to do this cheaper
pub fn replace_aliases(shell: &Shell, tokens: &mut Vec<String>) {
    if let Some(string) = shell.get_alias(&tokens[0]) {
        let replacements: Vec<String> = string.split(" ")
            .map(|s| s.to_string())
            .collect();
        println!("{:?}", replacements);
        let mut split_off = tokens.split_off(0);
        split_off.remove(0);
        tokens.extend(replacements.clone());
        tokens.extend(split_off);
        println!("{:?}", tokens);
    }
}

pub fn needs_substitution(test: &str) -> bool {
    let re_backtick = Regex::new(r"`[ >&|\-a-zA-Z0-9]+`").unwrap();
    let re_parenths = Regex::new(r"\$\([ >&|\-a-zA-Z0-9]+\)").unwrap();

    println!("{}", re_backtick.is_match(test));
    println!("{}", re_parenths.is_match(test));

    re_backtick.is_match(test) || re_parenths.is_match(test)
}

// This command is gonna be sooo fucking slow
pub fn substitute_commands(shell: &mut Shell, mut string: String) -> Result<String, CmdSubError> {
    let re_backtick = Regex::new(r"`[ >&|\-a-zA-Z0-9]+`").unwrap();
    let re_parenths = Regex::new(r"\$\([ >&|\-a-zA-Z0-9]+\)").unwrap();
    let mut outputs = Vec::<String>::new();
    if let Some(bt_captures) = re_backtick.captures(&string) {
        println!("command matched");
        for capture in bt_captures.iter() {
            if let Some(cmdmatch) = capture {
                let mut newstring = cmdmatch.as_str()[1..].to_string();
                newstring.pop();
                match Lexer::tokenize(shell, newstring, true) {
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
        shell.add_variable(String::from("$hello"), String::from("wassup"));
        shell.add_variable(String::from("$what"), String::from("is this"));
        let mut test_vec = vec![
            String::from("goodbye"), 
            String::from("$hello"),
            String::from("i know you"),
            String::from("$what"),
            String::from("$wontwork"),
        ];
        expand_variables(&shell, &mut test_vec);
        assert_eq!(
            test_vec,
            vec![
                String::from("goodbye"),
                String::from("wassup"),
                String::from("i know you"),
                String::from("is this"),
                String::from(""),
            ]
        );
    }

    #[test]
    fn check_alias_replacement() {
        let mut shell = Shell::new();
        shell.add_alias(
            String::from("addpkg"), 
            String::from("sudo pacman -S")
        );
        shell.add_alias(
            String::from("yeet"),
            String::from("sudo pacman -Rs"),
        );
        let mut test_vec = vec![
            String::from("addpkg"),
            String::from("pacman"),
        ];
        replace_aliases(&shell, &mut test_vec);
        assert_eq!(
            test_vec,
            vec![
                String::from("sudo"),
                String::from("pacman"),
                String::from("-S"),
                String::from("pacman"),
            ]
        );
        let mut test_vec2 = vec![
            String::from("yeet"),
            String::from("pacman"),
        ];
        replace_aliases(&shell, &mut test_vec2);
        assert_eq!(
            test_vec2,
            vec![
                String::from("sudo"),
                String::from("pacman"),
                String::from("-Rs"),
                String::from("pacman"),
            ]
        );
        let test_vec3 = vec![
            String::from("cogsy"),
            String::from("listen"),
            String::from("Your mother"),
        ];
        replace_aliases(&shell, &mut test_vec2);
        assert_eq!(
            test_vec3,
            vec![
                String::from("cogsy"),
                String::from("listen"),
                String::from("Your mother"),
            ]
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
}