use std::collections::HashMap;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::os::unix::io::IntoRawFd;

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
    pub fn add_alias(&mut self, key: String, value: String) {
        self.aliases.insert(key, value);
    }
    pub fn has_alias(&mut self, key: &str) -> bool {
        self.aliases.contains_key(key)  
    }
    /// Called by the alias builtin.
    pub fn remove_alias(&mut self, key: &str) {
        self.aliases.remove(key);
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

//TODO: Command and variable expansion