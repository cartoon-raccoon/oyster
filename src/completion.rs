use std::path::PathBuf;
use std::env;

use linefeed::complete::{
    Completion, 
    Completer, 
    Suffix
};
use linefeed::terminal::Terminal;
use linefeed::prompter::Prompter;

// First implementation of the completer.
// It does nothing as of yet, but it should work soon.
pub struct OshComplete {

}

impl<Term: Terminal> Completer<Term> for OshComplete {
    fn complete(
        &self,
        word: &str,
        _prompter: &Prompter<Term>,
        start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {
        if start == 0 {
            return Some(find_match_in_path(word))
        }
        Some(Vec::new())
    }
}

fn find_match_in_path(command: &str) -> Vec<Completion> {
    let mut res = Vec::new();
    let paths: Vec<PathBuf> = env::var("PATH")
        .unwrap_or(String::new())
        .split(":")
        .map(|n| PathBuf::from(n))
        .collect();
    if paths.is_empty() {
        return res;
    }
    for path in paths {
        //iterating over all the entries in the path
        for item in match std::fs::read_dir(path) {
            Ok(rd) => rd, 
            Err(_) => return res
        } {
            let item = match item {
                Ok(i) => i,
                Err(_) => continue
            };
            //getting the file name of the entry path
            if let Some(entry) = item.path().file_name() {
                let entry = match entry.to_str() {Some(st) => st, None => continue };
                if entry.starts_with(command) {
                    res.push(Completion {
                        completion: entry.to_string(),
                        display: None,
                        suffix: Suffix::Default,
                    });
                } else {
                    continue
                }
            } else {
                continue
            }
        }
    }
    res
}