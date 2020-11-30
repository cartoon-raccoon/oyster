use std::path::PathBuf;
use std::env;

use linefeed::complete::{
    Completion, 
    Completer, 
    Suffix
};
use linefeed::terminal::Terminal;
use linefeed::prompter::Prompter;

use crate::parser::Lexer;
use crate::types::TokenizeResult;

// First implementation of the completer.
// It does nothing as of yet, but it should work soon.
pub struct OshComplete {

}

impl<Term: Terminal> Completer<Term> for OshComplete {
    fn complete(
        &self,
        word: &str,
        prompter: &Prompter<Term>,
        start: usize,
        _end: usize,
    ) -> Option<Vec<Completion>> {

        match Lexer::tokenize(prompter.buffer()) {
            Ok(res) => {
                match res {
                    TokenizeResult::EndsOnAnd |
                    TokenizeResult::EndsOnOr |
                    TokenizeResult::EndsOnPipe => {
                        return Some(complete_bin(word))
                    }
                    TokenizeResult::UnmatchedCmdSub |
                    TokenizeResult::UnmatchedSqBrkt |
                    TokenizeResult::UnmatchedBQuote => {
                        return None
                    }
                    TokenizeResult::UnmatchedSQuote(s) |
                    TokenizeResult::UnmatchedDQuote(s) => {
                        return Some(complete_path(&s, true))
                    }
                    _ => {}
                }
            }
            Err(_) => return None
        }

        if word.starts_with("~") || word.contains("/") {
            return Some(complete_path(word, false))
        } else if word.starts_with("$") {
            return Some(complete_env(word))
        } else if start == 0 {
            return Some(complete_bin(word))
        } else {
            return Some(complete_path(word, false))
        }
    }
}

fn complete_bin(command: &str) -> Vec<Completion> {
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

fn complete_path(_path: &str, _quote: bool) -> Vec<Completion> {
    unimplemented!()
}

fn complete_env(_env: &str) -> Vec<Completion> {
    unimplemented!()
}