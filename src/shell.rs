use std::collections::HashMap;

pub struct Shell {
    jobs: Vec<String>,
    aliases: HashMap<String, String>,
    env: HashMap<String, String>,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            jobs: Vec::new(),
            aliases: HashMap::new(),
            env: HashMap::new(),
        }
    }
}