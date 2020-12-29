use std::iter::Peekable;
use std::str::Chars;

use crate::types::{
    Redirect,
    Exec,
    Token,
    TokenCmd,
    Job,
    ParseError,
    ParseResult,
    TokenizeResult,
    Quote,
};
use crate::shell::Shell;

use crate::expansion::{
    expand_variables,
    expand_glob,
    expand_tilde,
    replace_aliases,
    substitute_commands,
};

type CharsIter<'a> = Peekable<Chars<'a>>;

const METACHARS: [char; 10] = ['"', '&', '|', '\'', '>', '!', ';', '[', ']', ' '];

pub struct Lexer;

impl Lexer {

    /// Tokenizes the &str into a Vec of tokens
    /// 
    pub fn new() -> Self {
        Self {}
    }

    pub fn tokenize(cmd: &str) -> TokenizeResult {
        let mut chars = cmd.chars().peekable();
        let mut lexer = Self::new();

        let mut tokens = Vec::<Token>::new();
        let mut buffer = String::new();

        while let Some(c) = chars.next() {
            match c {
                '|' if chars.peek() == Some(&'|') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::Or);
                    chars.next();
                }
                '|' if chars.peek() == Some(&'&') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::Pipe2);
                    chars.next();
                }
                '|' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::Pipe);
                }
                '&' if chars.peek() == Some(&'&') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::And);
                    chars.next();
                }
                '&' if chars.peek() == Some(&'>') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::RDStdOutErr);
                    chars.next();
                }
                '&' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::Background);
                }
                '>' if chars.peek() == Some(&'>') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::RDAppend);
                    chars.next();
                }
                '>' if chars.peek() == Some(&'&') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::RDFileDesc);
                    chars.next();
                }
                '>' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::Redirect);
                }
                '<' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::RDStdin);
                }
                ';' | '\n' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::Consec);
                }
                '"' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match lexer.consume_dquote(&mut chars) {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                '\'' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match lexer.consume_squote(&mut chars) {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                '`' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match lexer.consume_bquote(&mut chars) {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                '[' if buffer.is_empty() => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match lexer.consume_sqbrkt(&mut chars) {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                '{' => {
                    buffer.push(c);
                    lexer.consume_brace(&mut buffer, &mut chars);
                    tokens.push(Token::Brace(buffer.clone()));
                    buffer.clear();
                }
                ' ' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                }
                '\\' => {
                    if let Some(c) = chars.next() {
                        buffer.push(c);
                    }
                }
                n @ '@' | n @ '$' if chars.peek() == Some(&'(') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match lexer.consume_cmdsub(n, &mut chars) {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                '$' if chars.peek() == Some(&'{') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match lexer.consume_nmspce(&mut chars) {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }

                // todo: match this for arrays as well
                '$' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match lexer.consume_variable(&mut chars) {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                _ => { buffer.push(c); }
            }
        }
        if !buffer.is_empty() {
            tokens.push(Token::Word(buffer))
        }

        //filtering empty words
        let mut tokens: Vec<Token> = tokens.into_iter()
            .filter(|token| {
                if let Token::Word(word) = token {
                    return !word.is_empty()
                } else if let Token::Brace(word) = token {
                    return !word.is_empty()
                }
                true
            }).collect();
        
        if let Some(token) = tokens.pop() {
            if token != Token::Consec {
                tokens.push(token);
            }
        }

        match tokens.last() {
            Some(&Token::Pipe) | Some(&Token::Pipe2) => {
                return TokenizeResult::EndsOnPipe
            }
            Some(&Token::And) => {
                return TokenizeResult::EndsOnAnd
            }
            Some(&Token::Or) => {
                return TokenizeResult::EndsOnOr
            }
            Some(_) => {
                return TokenizeResult::Good(tokens)
            }
            None => {
                return TokenizeResult::EmptyCommand
            }
        }
    }

    fn consume_dquote(&mut self, chars: &mut CharsIter) -> Result<Token, TokenizeResult> {
        let mut buf = String::new();
        loop {
            if let Some(c) = chars.next() {
                if c == '\\' {
                    if let Some(c) = chars.next() {
                        buf.push(c);
                        continue
                    }
                } else if c == '"' {
                    return Ok(Token::DQuote(buf))
                } else {
                    buf.push(c);
                }
            } else {
                return Err(TokenizeResult::UnmatchedDQuote(buf));
            }
        }
    }

    fn consume_squote(&mut self, chars: &mut CharsIter) -> Result<Token, TokenizeResult> {
        let mut buf = String::new();
        loop {
            if let Some(c) = chars.next() {
                if c == '\'' {
                    break
                } else {
                    buf.push(c);
                }
            } else {
                return Err(TokenizeResult::UnmatchedSQuote(buf));
            }
        }
        Ok(Token::SQuote(buf))
    }

    fn consume_bquote(&mut self, chars: &mut CharsIter) -> Result<Token, TokenizeResult> {
        let mut buf = String::new();
        loop {
            if let Some(c) = chars.next() {
                if c == '\\' {
                    if let Some(c) = chars.next() {
                        buf.push(c);
                        continue
                    }
                } else if c == '`' {
                    break
                } else {
                    buf.push(c);
                }
            } else {
                return Err(TokenizeResult::UnmatchedBQuote);
            }
        }
        Ok(Token::BQuote(buf))
    }

    fn consume_cmdsub(&mut self, prefix: char, chars: &mut CharsIter) -> Result<Token, TokenizeResult> {
        let mut buf = format!("{}", prefix);
        let mut nesting_level = 0;
        loop {
            if let Some(c) = chars.next() {
                if c == '@' || c == '$' && chars.peek() == Some(&'(') {
                    nesting_level += 1;
                } else if c == ')' {
                    if nesting_level > 0 {
                        nesting_level -= 1;
                    }
                    if nesting_level <= 0 {
                        buf.push(c);
                        break
                    }
                }
                buf.push(c);
            } else {
                return Err(TokenizeResult::UnmatchedCmdSub)
            }
        }
        Ok(Token::CmdSub(buf))
    }

    fn consume_variable(&mut self, chars: &mut CharsIter) -> Result<Token, TokenizeResult> {
        let mut buf = String::from("$");
        loop {
            if let Some(c) = chars.next() {
                buf.push(c);
                if let Some(&c) = chars.peek() {
                    if METACHARS.contains(&c) || !c.is_alphanumeric() {
                        break
                    }
                }
            } else {
                break
            }
        }
        Ok(Token::Variable(buf))
    }

    fn consume_brace(&mut self, buf: &mut String, chars: &mut CharsIter) {
        loop {
            if let Some(c) = chars.next() {
                buf.push(c);
                if let Some(&c) = chars.peek() {
                    if METACHARS.contains(&c) {
                        return
                    }
                }
            } else {
                return
            }
        }
    }

    fn consume_sqbrkt(&mut self, chars: &mut CharsIter) -> Result<Token, TokenizeResult> {
        let mut buf = String::new();
        let mut nesting_level = 0;
        loop {
            if let Some(c) = chars.next() {
                if c == '[' {
                    nesting_level += 1;
                } else if c == ']' {
                    if nesting_level > 0 {
                        nesting_level -= 1;
                    }
                    if nesting_level <= 0 {
                        break
                    }
                } else {
                    buf.push(c)
                }
            } else {
                return Err(TokenizeResult::UnmatchedSqBrkt)
            }
            
        }
        return Ok(Token::SqBrkt(buf))
    }

    fn consume_nmspce(&mut self, chars: &mut CharsIter) -> Result<Token, TokenizeResult> {
        let mut buf = String::new();
        loop {
            if let Some(c) = chars.next() {
                if c == '}' {
                    buf.push(c);
                    break
                } else {
                    buf.push(c)
                }
            } else {
                return Err(TokenizeResult::UnmatchedNmspce)
            }
        }
        Ok(Token::NmSpce(buf))
    }

    /// Splits command and parses special characters
    pub fn parse_tokens(shell: &mut Shell, tokens: Vec<Token>) 
    -> Result<ParseResult, ParseError> {
        
        let mut job_id = 1;
        let mut commandmap = Vec::<Vec<Token>>::new();
        let mut buffer: Vec<Token> = Vec::new();

        let mut stack: Vec<ParseResult> = Vec::new();

        //split token stream by command delimiters And, Or, Consec
        for token in tokens {
            match token {
                Token::And | 
                Token::Or |
                Token::Consec |
                Token::Background => {
                    buffer.push(token);
                    commandmap.push(buffer.clone());
                    buffer.clear();
                }
                _ => {
                    buffer.push(token);
                }
            }
        }
        if buffer.len() > 0 {
            commandmap.push(buffer);
        }

        //building job set
        let mut jobs = Vec::<Job>::new();

        match commandmap[0][0] {
            Token::And => {return Err(ParseError::StartsOnAnd);}
            Token::Or => {return Err(ParseError::StartsOnOr);}
            Token::Consec => {return Err(ParseError::StartsOnConsec);}
            Token::Pipe | Token::Pipe2 => {
                return Err(ParseError::PipeMismatch);
            }
            _ => {}
        }

        for mut tokengrp in commandmap {

            //* trackers
            let mut all_to_filename = false;
            let mut rd_to_filename = false;
            let mut rd_to_filedesc = false;
            let mut rd_from_stdin = false;

            let mut cmd_idx = 0;

            //* accumulators
            let mut buffer = Vec::<(Quote, String)>::new();
            let mut redirect = [String::new(), String::new(), String::new()];
            let mut redirects = Vec::<[String; 3]>::new();

            //* building job from these
            let mut cmds = Vec::<TokenCmd>::new();
            let mut execif = None;
            
            // alias expansion here, first word in group
            if let Token::Word(string) = &tokengrp[0] {
                if shell.has_alias(string) {
                    let string2 = string.clone();
                    let mut tail = tokengrp.split_off(0);
                    tail.remove(0);
                    let replacement = replace_aliases(shell, string2);

                    //aliasing only works if the alias value is a valid command
                    //so we don't have to match all cases here
                    if let TokenizeResult::Good(tokens) = 
                        Lexer::tokenize(&replacement) {
                        tokengrp.extend(tokens);
                        tokengrp.extend(tail);
                    }
                }
            }
            //building each pipeline
            for token in tokengrp {

                // println!("=========================");
                // println!("{}, {}, {}", all_to_filename, rd_to_filename, rd_to_filedesc);
                // println!("Token: {:?}", token);
                // println!("Buffer: {:?}", buffer);
                // println!("Cmds: {:?}", cmds);
                // println!("Redirect: {:?}", redirect);
                // println!("Redirects: {:?}", redirects);

                if rd_to_filename {
                    match token {
                        Token::Word(mut dest) => {
                            expand_variables(shell, &mut dest);
                            expand_tilde(shell, &mut dest);
                            redirect[2] = dest;
                            redirects.push(redirect.clone());
                            rd_to_filename = false;
                            continue;
                        }
                        Token::DQuote(mut dest) => {
                            expand_variables(shell, &mut dest);
                            let dest = substitute_commands(shell, &dest)?;
                            redirect[2] = dest;
                            redirects.push(redirect.clone());
                            rd_to_filename = false;
                            continue;
                        }
                        Token::SQuote(dest) => {
                            redirect[2] = dest;
                            redirects.push(redirect.clone());
                            rd_to_filename = false;
                            continue;
                        }
                        Token::CmdSub(cmd) => {
                            let dest = substitute_commands(shell, &cmd)?;
                            redirect[2] = dest;
                            redirects.push(redirect.clone());
                            rd_to_filename = false;
                            continue;
                        }
                        Token::BQuote(mut cmd) => {
                            expand_variables(shell, &mut cmd);
                            let dest = substitute_commands(shell, &cmd)?;
                            redirect[2] = dest;
                            redirects.push(redirect.clone());
                            rd_to_filename = false;
                            continue;
                        }
                        _ => { //* FIXME: Does not account for cases like:
                               //* 2>>&1 (In zsh this appends stderr to a file called 1)
                            return Err(ParseError::InvalidFileRD);
                        }
                    }
                } else if rd_to_filedesc {
                    match token {
                        Token::Word(dest) | 
                        Token::DQuote(dest) | 
                        Token::SQuote(dest) => {
                            redirect[2] = if dest == "1" {
                                String::from("&1")
                            } else if dest == "2" {
                                String::from("&2")
                            } else {
                                return Err(ParseError::InvalidFileDesc)
                            };
                            redirects.push(redirect.clone());
                            rd_to_filedesc = false;
                            continue;
                        }
                        _ => {
                            return Err(ParseError::InvalidFileRD);
                        }
                    }
                //TODO: Appending not yet implemented; only supports truncation
                } else if all_to_filename {
                    match token {
                        Token::Word(mut dest) => {
                            expand_variables(shell, &mut dest);
                            expand_tilde(shell, &mut dest);
                            redirects.push([String::from("1"), 
                                            String::from(">"), 
                                            String::from(dest.clone())]);
                            redirects.push([String::from("2"),
                                            String::from(">"),
                                            String::from(dest)]);
                            all_to_filename = false;
                            continue;
                        }
                        Token::DQuote(mut dest) => {
                            expand_variables(shell, &mut dest);
                            let dest = substitute_commands(shell, &dest)?;
                            redirects.push([String::from("1"), 
                                            String::from(">"), 
                                            String::from(dest.clone())]);
                            redirects.push([String::from("2"),
                                            String::from(">"),
                                            String::from(dest)]);
                            all_to_filename = false;
                            continue;
                        }
                        Token::SQuote(dest) => {
                            redirects.push([String::from("1"), 
                                            String::from(">"), 
                                            String::from(dest.clone())]);
                            redirects.push([String::from("2"),
                                            String::from(">"),
                                            String::from(dest)]);
                            all_to_filename = false;
                            continue;
                        }
                        _ => {
                            return Err(ParseError::InvalidFileRD);
                        }
                    }
                } else if rd_from_stdin {
                    match token {
                        Token::Word(mut dest) => {
                            expand_variables(shell, &mut dest);
                            expand_tilde(shell, &mut dest);
                            redirects.push([String::from(dest), 
                                            String::from("<"), 
                                            String::from("0")]);
                            rd_from_stdin = false;
                            continue;
                        }
                        Token::DQuote(mut dest) => {
                            expand_variables(shell, &mut dest);
                            let dest = substitute_commands(shell, &dest)?;
                            redirects.push([String::from(dest), 
                                            String::from("<"), 
                                            String::from("0")]);
                            rd_from_stdin = false;
                            continue;
                        }
                        Token::SQuote(dest) => {
                            redirects.push([String::from(dest), 
                                            String::from("<"), 
                                            String::from("0")]);
                            rd_from_stdin = false;
                            continue;
                        }
                        _ => {
                            return Err(ParseError::InvalidFileRD);
                        }
                    }
                }
                match token {
                    pipe @ Token::Pipe | pipe @ Token::Pipe2 => {
                        let mut final_redirects = 
                            Vec::<(String, Redirect, String)>::new();
                        for redirect in &redirects {
                            let redirecttype = 
                                if redirect[1] == ">>" {Redirect::Append}
                                else if redirect[1] == ">" {Redirect::Override}
                                else {Redirect::FromStdin};
                            final_redirects.push((redirect[0].clone(), 
                                                  redirecttype, 
                                                  redirect[2].clone()));
                        }
                        redirects.clear();
                        if buffer.len() < 1 {
                            return Err(ParseError::EmptyCommand);
                        } else {
                            cmds.push(
                                TokenCmd {
                                    cmd: buffer[0].clone(),
                                    args: buffer.clone(),
                                    redirects: final_redirects,
                                    pipe_stderr: if pipe == Token::Pipe {false} else {true},
                                }
                            );
                            buffer.clear();
                        }
                    }
                    Token::Word(mut string) => {
                        if cmd_idx == 0 {
                            //println!("{:?}", stack);
                            match string.as_str() {
                                "func" => {
                                    if !stack.is_empty() {
                                        return Err(ParseError::FuncInShellConst)
                                    }
                                    stack.push(ParseResult::Func)
                                }
                                "for" => {
                                    stack.push(ParseResult::For);
                                }
                                "while" => {
                                    stack.push(ParseResult::While);
                                }
                                "if" => {
                                    stack.push(ParseResult::If);
                                }
                                n@ "elif" | n@ "else" => {
                                    if let Some(&ParseResult::If) = stack.last() {

                                    } else {
                                        return Err(ParseError::GenericError(n.to_string()))
                                    }
                                }
                                n@ "done" => {
                                    if let Some(&ParseResult::For) = stack.last() {
                                        stack.pop();
                                    } else if let Some(&ParseResult::While) = stack.last() {
                                        stack.pop();
                                    } else {
                                        return Err(ParseError::GenericError(n.to_string()))
                                    }
                                }
                                n@ "end" => {
                                    if let Some(&ParseResult::If) = stack.last() {
                                        stack.pop();
                                    } else {
                                        return Err(ParseError::GenericError(n.to_string()))
                                    }
                                }
                                n@ "endfn" => {
                                    if let Some(&ParseResult::Func) = stack.last() {
                                        stack.pop();
                                    } else {
                                        return Err(ParseError::GenericError(n.to_string()))
                                    }
                                }
                                _ => {}
                            }
                        }
                        if string.contains("*") {
                            expand_tilde(shell, &mut string);
                            buffer.extend(expand_glob(&string)?.into_iter().map(
                                |string| {
                                    (Quote::NQuote, string)
                                }
                            ).collect::<Vec<(Quote, String)>>());
                            continue;
                        }
                        buffer.push((Quote::NQuote, string));
                    }
                    Token::Variable(string) => {
                        buffer.push((Quote::Variable, string));
                    }
                    Token::DQuote(string) => {
                        buffer.push((Quote::DQuote, string));
                    }
                    Token::SQuote(string) => {
                        buffer.push((Quote::SQuote, string));
                    }
                    Token::BQuote(string) => {
                        buffer.push((Quote::BQuote, string));
                    }
                    Token::CmdSub(string) => {
                        buffer.push((Quote::CmdSub, string));
                    }
                    Token::SqBrkt(string) => {
                        buffer.push((Quote::SqBrkt, string));
                    }
                    Token::NmSpce(mut string) => {
                        string.pop();
                        buffer.push((Quote::NmSpce, string));
                    }
                    Token::Brace(string) => {
                        buffer.push((Quote::CBrace, string));
                    }
                    rd @ Token::Redirect |
                    rd @ Token::RDAppend |
                    rd @ Token::RDFileDesc => {
                        if rd == Token::RDFileDesc {
                            rd_to_filedesc = true;
                            rd_to_filename = false;
                        } else {
                            rd_to_filedesc = false;
                            rd_to_filename = true;
                        }
                        if let Some(fd) = buffer.pop() {
                            if fd.1 == "2" || fd.1 == "1" {
                                //origin is file descriptor
                                redirect[0] = fd.1;
                                redirect[1] = if rd == Token::RDAppend {
                                    String::from(">>")
                                } else {
                                    String::from(">")
                                }
                            } else {
                                //is part of the command
                                buffer.push(fd);
                                redirect[0] = String::from("1");
                                redirect[1] = if rd == Token::RDAppend {
                                    String::from(">>")
                                } else {
                                    String::from(">")
                                }
                            }
                        } else {
                            return Err(ParseError::InvalidRDSyntax);
                        }
                    }
                    Token::RDStdOutErr => {
                        rd_to_filedesc = false;
                        rd_to_filename = false;
                        all_to_filename = true;
                    }
                    Token::RDStdin => {
                        rd_to_filedesc = false;
                        rd_to_filename = false;
                        all_to_filename = false;
                        rd_from_stdin = true;
                    }
                    //* if matching on these, they must be the last item
                    Token::And => {
                        execif = Some(Exec::And);
                    }
                    Token::Or => {
                        execif = Some(Exec::Or);
                    }
                    Token::Consec => {
                        execif = Some(Exec::Consec);
                    }
                    Token::Background => {
                        execif = Some(Exec::Background);
                    }
                }
                cmd_idx += 1;
            }
            let mut final_redirects = 
                Vec::<(String, Redirect, String)>::new();
            for redirect in &redirects {
                let redirecttype = 
                if redirect[1] == ">>" {Redirect::Append}
                else if redirect[1] == ">" {Redirect::Override}
                else {Redirect::FromStdin};
                final_redirects.push((redirect[0].clone(), 
                                        redirecttype, 
                                        redirect[2].clone()));
            }
            redirects.clear();
            if buffer.len() < 1 {
                return Err(ParseError::EmptyCommand);
            } else {
                cmds.push(
                    TokenCmd {
                        cmd: buffer[0].clone(),
                        args: buffer.clone(),
                        redirects: final_redirects,
                        pipe_stderr: false,
                    }
                );
            }

            jobs.push(
                Job {
                    cmds: cmds,
                    execnext: execif,
                    id: job_id,
                }
            );
            job_id += 1;
        }

        //println!("{:?}", jobs);
        if stack.is_empty() {
            return Ok(ParseResult::Good(jobs))
        } else {
            // safe to unwrap because the stack is not empty
            return Ok(stack.pop().unwrap())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexing() {
        let test_string1 = "git add src/{core,shell}.rs && git $commit -m \"hello\" >> hello.txt";
        let test_string2 = "cowsay -f tux -W 80 < $(cat ~/Documents/stallman) | lolcat -p 0.8";
        let test_string3 = "echo $hello";

        match Lexer::tokenize(test_string1) {
            TokenizeResult::Good(tokens) => {
                let proper = vec![
                    Token::Word(String::from("git")),
                    Token::Word(String::from("add")),
                    Token::Brace(String::from("src/{core,shell}.rs")),
                    Token::And,
                    Token::Word(String::from("git")),
                    Token::Variable(String::from("$commit")),
                    Token::Word(String::from("-m")),
                    Token::DQuote(String::from("hello")),
                    Token::RDAppend,
                    Token::Word(String::from("hello.txt")),
                ];
                assert_eq!(tokens, proper)
            }
            n @ _ => {
                panic!( "{:?}", n)
            }
        }

        match Lexer::tokenize(test_string2) {
            TokenizeResult::Good(tokens) => {
                let proper = vec![
                    Token::Word(String::from("cowsay")),
                    Token::Word(String::from("-f")),
                    Token::Word(String::from("tux")),
                    Token::Word(String::from("-W")),
                    Token::Word(String::from("80")),
                    Token::RDStdin,
                    Token::CmdSub(String::from("$(cat ~/Documents/stallman)")),
                    Token::Pipe,
                    Token::Word(String::from("lolcat")),
                    Token::Word(String::from("-p")),
                    Token::Word(String::from("0.8")),
                ];
                assert_eq!(tokens, proper)
            }
            err @ _ => {
                panic!("{:?}", err)
            }
        }

        match Lexer::tokenize(test_string3) {
            TokenizeResult::Good(tokens) => {
                let proper = vec![
                    Token::Word(String::from("echo")),
                    Token::Variable(String::from("$hello")),
                ];

                assert_eq!(tokens, proper)
            }
            n @ _ => {
                panic!("{:?}", n)
            }
        }
    }
}
