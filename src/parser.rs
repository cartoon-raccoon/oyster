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

const METACHARS: [char; 10] = ['"', '&', '|', '\'', '>', '!', ';', '[', ']', ' '];

pub struct Lexer;

impl Lexer {

    /// Tokenizes the &str into a Vec of tokens
    /// 
    /// This is a very badly implemented FSM.
    pub fn tokenize(line: &str) 
    -> Result<TokenizeResult, ParseError> {
        //println!("{:?}", line);

        let mut line_iter = line.chars().peekable();

        //Accumulators
        let mut tokenvec = Vec::<Token>::new();
        let mut word = String::new();

        //Trackers
        //let mut in_var = false;
        let mut in_dquote = false;
        let mut in_squote = false;
        let mut in_bquote = false;
        let mut in_cmdsub = false;
        let mut in_sqbrkt = false;
        let mut in_nmespc = false;
        let mut brace_level: i32 = 0;
        let mut sbrkt_level: i32 = 0;
        let mut has_brace = false;
        let mut ignore_next = false;
        let mut prev_char = None;

        let push = |elements: &mut Vec<Token>, 
                    charvec: &mut String,
                    character: char,
                    in_double_quotes: bool,
                    in_single_quotes: bool,
                    has_brace: bool,| {
            if !in_double_quotes && !in_single_quotes {
                if has_brace {
                    elements.push(Token::Brace(charvec.clone()));
                } else {
                    if !charvec.is_empty() {
                        elements.push(
                            Token::Word(charvec.trim().to_string())
                        );
                    }
                }
                charvec.clear();
            } else {
                charvec.push(character)
            }
        };
        
        //* Phase 1: Tokenisation
        while let Some(c) = line_iter.next() {
            // println!("========================");
            // println!("{:?}", prev_char);
            // println!("{:?}", c);
            // println!("{:?}", line_iter.peek());
            // println!("{:?}", word);
            // println!("{:?}", tokenvec);
            // println!("Last item: {:?}", tokenvec.last());
            // println!("In Dquote: {}", in_dquote);
            // println!("In Squote: {}", in_squote);
            // println!("Ignore:    {}", ignore_next);
            if ignore_next {
                ignore_next = false;
                prev_char = Some(c);
                continue;
            }
            match c {
                '|' if line_iter.peek() == Some(&'|') 
                    && !in_squote && !in_bquote 
                    && !in_sqbrkt && !in_cmdsub && !in_nmespc => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        tokenvec.push(Token::Or);
                        ignore_next = true;
                    }
                    has_brace = false;
                },
                '|' if line_iter.peek() == Some(&'&') 
                    && !in_squote && !in_bquote 
                    && !in_sqbrkt && !in_cmdsub && !in_nmespc => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        tokenvec.push(Token::Pipe2);
                        ignore_next = true;
                    }
                    has_brace = false;
                },
                '|' if !in_squote && !in_bquote 
                    && !in_sqbrkt && !in_cmdsub && !in_nmespc => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        tokenvec.push(Token::Pipe);
                        ignore_next = false;
                    }
                    has_brace = false;
                },
                '&' if line_iter.peek() == Some(&'&') 
                    && !in_squote && !in_bquote 
                    && !in_sqbrkt && !in_cmdsub && !in_nmespc => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        tokenvec.push(Token::And);
                        ignore_next = true;
                    }
                    has_brace = false;
                },
                '&' if line_iter.peek() == Some(&'>') 
                    && !in_squote && !in_bquote 
                    && !in_sqbrkt && !in_cmdsub && !in_nmespc => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        tokenvec.push(Token::RDStdOutErr);
                        ignore_next = true;
                    }
                    has_brace = false;
                }
                '&' if !in_squote && !in_bquote 
                    && !in_sqbrkt && !in_cmdsub && !in_nmespc  => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        if prev_char == Some('>') {
                            tokenvec.push(Token::FileMarker);
                        } else {
                            tokenvec.push(Token::Background);
                        }
                        ignore_next = false;
                    }
                    has_brace = false;
                },
                '{' if !in_dquote && !in_squote && !in_nmespc
                    && !in_bquote && !in_sqbrkt && !in_cmdsub => {
                    has_brace = true;
                    brace_level += 1;
                    word.push(c);
                }
                '}' if !in_dquote && !in_squote 
                    && !in_bquote && !in_sqbrkt && !in_cmdsub => {
                    if brace_level > 0 {}
                    brace_level -= 1;
                    word.push(c);
                    if brace_level == 0 && !in_nmespc 
                    && line_iter.peek() == Some(&' ') {
                        tokenvec.push(Token::Brace(word.clone()));
                        word.clear();
                    } else if in_nmespc {
                        tokenvec.push(Token::NmSpce(word.clone()));
                        word.clear();
                    }
                }
                '[' if !in_dquote && !in_squote && !in_bquote 
                    && !in_nmespc && !in_cmdsub => {
                    sbrkt_level += 1;
                    if prev_char == Some(' ') || prev_char == None {
                        push(&mut tokenvec, &mut word, c, 
                            in_dquote, in_squote, has_brace
                        );
                        sbrkt_level = 1;
                        in_sqbrkt = true;
                    } else {
                        word.push(c);
                    }
                }
                ']' if !in_squote && !in_bquote && !in_nmespc
                    && !in_dquote &&  in_sqbrkt && !in_cmdsub => {
                    sbrkt_level -= 1;
                    if sbrkt_level == 0 && in_sqbrkt {
                        tokenvec.push(Token::SqBrkt(word.clone()));
                        word.clear();
                        in_sqbrkt = false;
                    } else {
                        word.push(c);
                    }
                }
                n@ '@' | n@ '$' if line_iter.peek() == Some(&'(')
                    && !in_squote && !in_cmdsub && !in_nmespc
                    && !in_sqbrkt && !in_bquote => {
                    if n == '@' {
                        word.push_str("@(");
                    } else if n == '$' {
                        word.push_str("$(");
                    }
                    has_brace = false;
                    ignore_next = true;
                    if !in_dquote {
                        in_cmdsub = true;
                    }
                }
                '$' if line_iter.peek() == Some(&'{')
                    && !in_squote && !in_cmdsub && !in_dquote
                    && !in_sqbrkt && !in_bquote && !in_nmespc => {
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_nmespc {
                        in_nmespc = true;
                        ignore_next = true;
                    }
                }
                ')' if !in_squote && !in_bquote && !in_nmespc
                    && !in_sqbrkt && in_cmdsub => {
                    word.push(')');
                    if !in_dquote {
                        tokenvec.push(Token::CmdSub(word.clone()));
                        word.clear();
                        in_cmdsub = false;
                    }
                }
                '>' if line_iter.peek() == Some(&'>') 
                    && !in_squote && !in_bquote && !in_nmespc
                    && !in_sqbrkt && !in_cmdsub => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        tokenvec.push(Token::RDAppend);
                        ignore_next = true;
                    }
                    has_brace = false;
                },
                '>' if line_iter.peek() == Some(&'&') 
                    && !in_squote && !in_bquote && !in_nmespc
                    && !in_sqbrkt && !in_dquote && !in_cmdsub => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        tokenvec.push(Token::RDFileDesc);
                        ignore_next = true;
                    }
                    has_brace = false;
                }
                '>' if line_iter.peek() != Some(&'>') 
                    && !in_squote && !in_bquote && !in_nmespc
                    && !in_sqbrkt && !in_dquote && !in_cmdsub => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        tokenvec.push(Token::Redirect);
                        ignore_next = false;
                    }
                    has_brace = false;
                },
                '<' if !in_squote && !in_bquote && !in_nmespc
                    && !in_sqbrkt && !in_dquote && !in_cmdsub => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        tokenvec.push(Token::RDStdin);
                        ignore_next = false;
                    }
                    has_brace = false;
                },
                ';' if !in_squote && !in_bquote && !in_nmespc
                    && !in_sqbrkt && !in_dquote && !in_cmdsub => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if let Some(&Token::Consec) = tokenvec.last() {
                    } else {
                        tokenvec.push(Token::Consec);
                        ignore_next = false;
                    }
                    has_brace = false;
                }
                '\n' if !in_squote && !in_bquote && !in_nmespc
                    && !in_sqbrkt && !in_dquote && !in_cmdsub => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if let Some(token) = tokenvec.last()  {
                        match token {
                            Token::Consec |
                            Token::FileMarker |
                            Token::Or |
                            Token::And |
                            Token::Background |
                            Token::Pipe |
                            Token::Pipe2 => {}
                            n@ Token::Redirect |
                            n@ Token::RDStdin |
                            n@ Token::RDAppend |
                            n@ Token::RDStdOutErr |
                            n@ Token::RDFileDesc => {
                                return Err(ParseError::GenericError(n.to_string()))
                            }
                            _ => {
                                tokenvec.push(Token::Consec);
                                ignore_next = false;
                            }
                        }
                    }
                    has_brace = false;
                }
                '`' if !in_squote && !in_sqbrkt && !in_cmdsub && !in_nmespc => {
                    has_brace = false;
                    word.push(c);
                    if in_bquote {
                        in_bquote = false;
                        if !in_dquote {
                            tokenvec.push(Token::BQuote(word.clone()));
                            word.clear();
                        }
                    } else {
                        in_bquote = true;
                    }
                }
                '\'' => {
                    ignore_next = false;
                    if in_squote && !in_dquote  {
                        in_squote = false;
                        tokenvec.push(Token::SQuote(word.clone()));
                        word.clear();
                    } else if in_dquote {
                        word.push(c);
                    } else if in_bquote || in_cmdsub {
                        word.push(c);
                    } else if brace_level > 0 {

                    } else {
                        push(&mut tokenvec, &mut word, c, 
                            in_dquote, in_squote, has_brace
                        );
                        in_squote = true;
                    }
                }
                '"' if !in_squote && !in_bquote && !in_nmespc
                    && !in_sqbrkt && !in_cmdsub => {
                    ignore_next = false;
                    if in_dquote {
                        in_dquote = false;
                        if brace_level > 0 {

                        } else {
                            tokenvec.push(Token::DQuote(word.clone()));
                            word.clear();
                        }
                    } else {
                        push(&mut tokenvec, &mut word, c, 
                            in_dquote, in_squote, has_brace
                        );
                        in_dquote = true;
                    }
                }
                '\\' if !in_squote => {
                    if in_squote {
                        word.push(c);
                    } else {
                        if has_brace && brace_level != 0 {
                            // if line_iter.peek() == Some(&'{')
                            // || line_iter.peek() == Some(&'}') {
                            //     word.push(c);
                            // }
                            word.push(c);
                        }
                        if let Some(ch) = line_iter.next() {
                            word.push(ch);
                            prev_char = Some(ch);
                        }
                        continue;
                    }
                }
                ' ' if !in_squote && !in_bquote && !in_nmespc
                    && !in_sqbrkt && !in_cmdsub => {
                    ignore_next = false;
                    if prev_char != Some(' ') {
                        if !in_dquote {
                            if has_brace {
                                if !word.is_empty() {
                                    tokenvec.push(Token::Brace(word.clone()));
                                    has_brace = false;
                                }
                            } else {
                                if !word.is_empty() {
                                    tokenvec.push(Token::Word(word.clone())); 
                                }
                            }
                            word.clear();
                        } else {
                        word.push(c);
                        }
                    }
                }
                '#' if !in_squote && !in_bquote && !has_brace => {
                    if in_dquote || in_sqbrkt {
                        word.push(c);
                    } else {
                        break;
                    }
                }
                _ => {
                    ignore_next = false;
                    word.push(c);
                }
            }
            prev_char = Some(c)
        }
        if brace_level > 0 || has_brace {
            tokenvec.push(Token::Brace(word.clone()));
        } else {
            if !word.is_empty() {
                tokenvec.push(Token::Word(word.clone()));
            }
        }

        //filtering empty words
        let mut tokenvec: Vec<Token> = tokenvec.into_iter()
            .filter(|token| {
                if let Token::Word(word) = token {
                    return !word.is_empty()
                } else if let Token::Brace(word) = token {
                    return !word.is_empty()
                }
                true
            }).collect();
        
        if let Some(token) = tokenvec.pop() {
            if token != Token::Consec {
                tokenvec.push(token);
            }
        }
    
        //println!("{:?}", tokenvec);
        
        if in_bquote {
            return Ok(TokenizeResult::UnmatchedBQuote);
        } else if in_cmdsub {
            return Ok(TokenizeResult::UnmatchedCmdSub);
        } else if in_sqbrkt {
            return Ok(TokenizeResult::UnmatchedSqBrkt);
        } else if in_squote {
            return Ok(TokenizeResult::UnmatchedSQuote(word)); 
        } else if in_dquote {
            return Ok(TokenizeResult::UnmatchedDQuote(word));
        } else {
            match tokenvec.last() {
                Some(token) => {
                    match *token {
                        Token::Or => {
                            return Ok(TokenizeResult::EndsOnOr);
                        }
                        Token::And => {
                            return Ok(TokenizeResult::EndsOnAnd);
                        }
                        Token::Pipe | Token::Pipe2 => {
                            return Ok(TokenizeResult::EndsOnPipe);
                        }
                        _ => {}
                    }
                }
                None => {
                    return Ok(TokenizeResult::EmptyCommand);
                }
            }
        }

        if tokenvec.len() > 0 {
            Ok(TokenizeResult::Good(tokenvec))
        } else {
            Ok(TokenizeResult::EmptyCommand)
        }
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
                        Lexer::tokenize(&replacement)? {
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
                    _ => {
                        
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

// building a better tokenizer
// the parser is fine as is rn

pub struct LexerNew<'a> {
    inner: Peekable<Chars<'a>>,
}

impl<'a> LexerNew<'a> {
    pub fn new() -> Self {
        Self {
            inner: "".chars().peekable(),
        }
    }

    pub fn tokenize(&mut self, cmd: &'a str) -> TokenizeResult {
        self.inner = cmd.chars().peekable();

        let mut tokens = Vec::<Token>::new();
        let mut buffer = String::new();
        let mut prev_char = None;

        while let Some(c) = self.inner.next() {
            match c {
                '|' if self.inner.peek() == Some(&'|') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::Or);
                    self.inner.next();
                }
                '|' if self.inner.peek() == Some(&'&') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::Pipe2);
                    self.inner.next();
                }
                '|' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::Pipe);
                }
                '&' if self.inner.peek() == Some(&'&') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::And);
                    self.inner.next();
                }
                '&' if self.inner.peek() == Some(&'>') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::RDStdOutErr);
                    self.inner.next();
                }
                '&' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::Background);
                }
                '>' if self.inner.peek() == Some(&'>') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::RDAppend);
                    self.inner.next();
                }
                '>' if self.inner.peek() == Some(&'&') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    tokens.push(Token::RDFileDesc);
                    self.inner.next();
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
                    match self.consume_dquote() {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                '\'' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match self.consume_squote() {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                '`' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match self.consume_bquote() {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                '[' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match self.consume_sqbrkt() {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                '{' => {
                    buffer.push(c);
                    self.consume_brace(&mut buffer);
                    tokens.push(Token::Brace(buffer.clone()));
                    buffer.clear();
                }
                ' ' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                }
                '\\' => {
                    if let Some(c) = self.inner.next() {
                        buffer.push(c);
                    }
                }
                n @ '@' | n @ '$' if self.inner.peek() == Some(&'(') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match self.consume_cmdsub(n) {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                '$' if self.inner.peek() == Some(&'{') => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    // consume nmspce
                }

                // todo: match this for arrays as well
                '$' => {
                    tokens.push(Token::Word(buffer.clone()));
                    buffer.clear();
                    match self.consume_variable() {
                        Ok(tk) => tokens.push(tk),
                        Err(e) => return e
                    }
                }
                _ => { buffer.push(c); }
            }
            prev_char = Some(c);
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

    fn consume_dquote(&mut self) -> Result<Token, TokenizeResult> {
        let mut buf = String::new();
        loop {
            if let Some(c) = self.inner.next() {
                if c == '\\' {
                    if let Some(c) = self.inner.next() {
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

    fn consume_squote(&mut self) -> Result<Token, TokenizeResult> {
        let mut buf = String::new();
        loop {
            if let Some(c) = self.inner.next() {
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

    fn consume_bquote(&mut self) -> Result<Token, TokenizeResult> {
        let mut buf = String::new();
        loop {
            if let Some(c) = self.inner.next() {
                if c == '\\' {
                    if let Some(c) = self.inner.next() {
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

    fn consume_cmdsub(&mut self, prefix: char) -> Result<Token, TokenizeResult> {
        let mut buf = format!("{}", prefix);
        let mut nesting_level = 0;
        loop {
            if let Some(c) = self.inner.next() {
                if c == '@' || c == '$' && self.inner.peek() == Some(&'(') {
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

    fn consume_variable(&mut self) -> Result<Token, TokenizeResult> {
        let mut buf = String::from("$");
        loop {
            if let Some(c) = self.inner.next() {
                buf.push(c);
                if let Some(&c) = self.inner.peek() {
                    if METACHARS.contains(&c) || !c.is_alphanumeric() {
                        break
                    }
                }
            }
        }
        Ok(Token::Variable(buf))
    }

    fn consume_brace(&mut self, buf: &mut String) {
        loop {
            if let Some(c) = self.inner.next() {
                buf.push(c);
                if let Some(&c) = self.inner.peek() {
                    if METACHARS.contains(&c) {
                        return
                    }
                }
            } else {
                return
            }
        }
    }

    fn consume_sqbrkt(&mut self) -> Result<Token, TokenizeResult> {
        let mut buf = String::from("[");
        let mut nesting_level = 0;
        loop {
            if let Some(c) = self.inner.next() {
                if c == ']' {
                    nesting_level += 1;
                } else if c == ']' {
                    if nesting_level > 0 {
                        nesting_level -= 1;
                    }
                    if nesting_level <= 0 {
                        buf.push(']');
                        break
                    }
                }
            } else {
                return Err(TokenizeResult::UnmatchedSqBrkt)
            }
            
        }
        return Ok(Token::SqBrkt(buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexing() {
        let test_string1 = "git add src/{core,shell}.rs && git $commit -m \"hello\" >> hello.txt";
        let test_string2 = "cowsay -f tux -W 80 < $(cat ~/Documents/stallman) | lolcat -p 0.8";
        let mut lexer = LexerNew::new();

        match lexer.tokenize(test_string1) {
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

        match lexer.tokenize(test_string2) {
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
    }
}
