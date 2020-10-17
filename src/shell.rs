use std::collections::HashMap;
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::types::Job;

pub struct Shell {
    jobs: BTreeMap<i32, Job>,
    aliases: HashMap<String, String>,
    env: HashMap<String, String>,
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