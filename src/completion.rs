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

        match Lexer::tokenize(&mut prompter.buffer().to_string()) {
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

fn complete_path(path: &str, quote: bool) -> Vec<Completion> {
    let mut path = path.to_string();
    expand_tilde(&mut path);
    if quote {
        path = path.replace(" ", "\\ ")
    }
    unimplemented!()
}

fn complete_env(_env: &str) -> Vec<Completion> {
    unimplemented!()
}

fn expand_tilde(string: &mut String) {  
    let home = env::var("HOME").unwrap_or(String::new());
    if home.is_empty() { return }
    let pwd = env::var("PWD").unwrap_or(String::new());
    if pwd.is_empty() { return }
    if string.starts_with("~") {
        if string.starts_with("~+") {
            *string = string.replacen("~+", &pwd, 1);
        } else {
            *string = string.replacen("~", &home, 1);
        }
    } else {
        return;
    }
}