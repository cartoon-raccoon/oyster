use std::env;

use regex::Regex;
use glob::glob;

use crate::shell::Shell;
use crate::parser::Lexer;
use crate::types::{
    ParseError,
    ParseResult,
    TokenizeResult::*,
    CmdSubError,
    ShellError,
};
use crate::execute;

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

pub fn expand_braces(shell: &mut Shell, mut string: String) -> Result<Vec<String>, ShellError> {
    if string.starts_with("{") && string.ends_with("}") && string.contains("..") {
        string.pop();
        return Ok(expand_range(shell, &string[1..])?)
    }
    let output = brace_expansion::tokenize(&string);
    Ok(brace_expansion::output(output))
}

pub fn expand_range(shell: &mut Shell, brkt: &str) -> Result<Vec<String>, ShellError> {
    let mut range: Vec<String> = brkt.split("..").filter(
        |string| !string.is_empty()
    ).map(|string| string.to_string()).collect();
    if range.len() < 2 || range.len() > 3 {
        //println!("{}", brkt);
        return Err(
            ShellError::from("oyster: error expanding range")
        )
    }
    let step_by = if range.len() == 3 {
        match range[2].parse::<u32>() {
            Ok(int) => int,
            Err(_) => {
                return Err(
                    ShellError::from("oyster: invalid argument in range")
                )
            }
        }
    } else {1};
    let up_to_equals = range[1].starts_with("=");
    if up_to_equals {
        let replace = range[1].replace("=", "");
        range[1] = replace;
    }
    let mut numeric = Vec::new();
    for mut number in range {
        if number.starts_with("$") {
            if let Some(num) = shell.get_variable(&number[1..]) {
                number = format!("{}", num)
            } else {
                return Err(ShellError::from("oyster: no variable in shell"))
            }
        }
        match number.parse::<i32>() {
            Ok(int) => {
                numeric.push(int);
            }
            Err(_) => {
                return Err(
                    ShellError::from("oyster: invalid argument in range")
                )
            }
        }
    }
    let (mut start, end) = (numeric[0], numeric[1]);
    let mut to_return = Vec::new();
    if start < end {
        while start < end {
            to_return.push(start.to_string());
            start += step_by as i32;
        }
    } else {
        while start > end {
            to_return.push(start.to_string());
            start -= step_by as i32;
        }
    }
    if up_to_equals {
        to_return.push(end.to_string());
    }
    Ok(to_return)
}

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
    use crate::types::Variable as Var;

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
}