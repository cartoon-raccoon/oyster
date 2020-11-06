use crate::types::{
    Redirect,
    Exec,
    Token,
    TokenCmd,
    Job,
    ParseError,
    TokenizeResult,
    Quote,
};
use crate::shell::{
    Shell,
    substitute_commands,
    expand_variables,
    expand_braces,
    expand_tilde,
    replace_aliases,
};

pub struct Lexer;

impl Lexer {

    /// Tokenizes the &str into a Vec of tokens
    pub fn tokenize(line: &str) 
    -> Result<TokenizeResult, ParseError> {

        let mut line_iter = line.chars().peekable();

        //Accumulators
        let mut tokenvec = Vec::<Token>::new();
        let mut word = String::new();

        //Trackers
        //let mut in_var = false;
        let mut in_dquote = false;
        let mut in_squote = false;
        let mut in_bquote = false;
        let mut in_sqbrkt = false;
        let mut brace_level: i32 = 0;
        let mut has_brace = false;
        let mut has_bquote = false;
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
                    elements.push(Token::Word(charvec.trim().to_string()));
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
            // println!("In tilde:  {}", in_tilde);
            if ignore_next {
                ignore_next = false;
                prev_char = Some(c);
                continue;
            }
            match c {
                '|' if line_iter.peek() == Some(&'|') 
                    && !in_squote && !in_bquote && !in_sqbrkt => {
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
                    && !in_squote && !in_bquote && !in_sqbrkt => {
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
                '|' if !in_squote && !in_bquote && !in_sqbrkt => {
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
                    && !in_squote && !in_bquote && !in_sqbrkt => {
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
                    && !in_squote && !in_bquote && !in_sqbrkt => {
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
                '&' if !in_squote && !in_bquote && !in_sqbrkt => {
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
                '{' if !in_dquote && !in_squote 
                    && !in_bquote && !in_sqbrkt => {
                    has_brace = true;
                    brace_level += 1;
                    word.push(c);
                }
                '}' if !in_dquote && !in_squote 
                    && !in_bquote && !in_sqbrkt => {
                    brace_level -= 1;
                    word.push(c);
                    if brace_level == 0 && line_iter.peek() == Some(&' ') {
                        tokenvec.push(Token::Brace(word.clone()));
                        word.clear();
                    }
                }
                '[' if !in_dquote && !in_squote && !in_bquote => {
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    in_sqbrkt = true;
                }
                ']' if !in_squote && !in_bquote && !in_dquote
                    && in_sqbrkt => {
                    tokenvec.push(Token::SqBrkt(word.clone()));
                    word.clear();
                    in_sqbrkt = false;
                }
                '>' if line_iter.peek() == Some(&'>') 
                    && !in_squote && !in_bquote && !in_sqbrkt => {
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
                    && !in_squote && !in_bquote 
                    && !in_sqbrkt && !in_dquote => {
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
                    && !in_squote && !in_bquote 
                    && !in_sqbrkt && !in_dquote => {
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
                '<' if !in_squote && !in_bquote 
                    && !in_sqbrkt && !in_dquote => {
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
                ';' | '\n' if !in_squote && !in_bquote && !in_sqbrkt => {
                    if brace_level > 0 {return Err(ParseError::MetacharsInBrace)}
                    push(&mut tokenvec, &mut word, c, 
                        in_dquote, in_squote, has_brace
                    );
                    if !in_dquote {
                        tokenvec.push(Token::Consec);
                        ignore_next = false;
                    }
                    has_brace = false;
                }
                '`' if !in_squote && !in_sqbrkt => {
                    has_brace = false;
                    word.push(c);
                    if in_bquote {
                        in_bquote = false;
                        if !in_dquote {
                            tokenvec.push(Token::BQuote(word.clone()));
                            word.clear();
                        } else {
                            has_bquote = true;
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
                    } else if in_bquote {
                        word.push(c);
                    } else if brace_level > 0 {

                    } else {
                        in_squote = true;
                    }
                }
                '"' if !in_squote && !in_bquote && !in_sqbrkt => {
                    ignore_next = false;
                    if in_dquote {
                        in_dquote = false;
                        if brace_level > 0 {

                        } else if has_bquote {
                            tokenvec.push(Token::BQuote(word.clone()));
                            word.clear();
                            has_bquote = false;
                        } else {
                            tokenvec.push(Token::DQuote(word.clone()));
                            word.clear();
                        }
                    } else {
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
                ' ' if !in_squote && !in_bquote && !in_sqbrkt => {
                    ignore_next = false;
                    if prev_char != Some(' ') {
                        if !in_dquote {
                            if has_brace {
                                tokenvec.push(Token::Brace(word.clone()));
                                has_brace = false;
                            } else {
                                tokenvec.push(Token::Word(word.clone())); 
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
            tokenvec.push(Token::Brace(word));
        } else {
            tokenvec.push(Token::Word(word));
        }

        // filtering empty words
        let mut tokenvec: Vec<Token> = tokenvec.into_iter()
            .filter(|token| {
                if let Token::Word(word) = token {
                    if word == "" {
                        return false
                    } else {
                        return true
                    }
                } else if let Token::Brace(word) = token {
                    if word == "" {
                        return false
                    } else {
                        return true
                    }
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
        } else if in_squote {
            return Ok(TokenizeResult::UnmatchedSQuote); 
        } else if in_dquote {
            return Ok(TokenizeResult::UnmatchedDQuote);
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
    pub fn parse_tokens(shell: &mut Shell, tokens: Vec<Token>) -> Result<Vec<Job>, ParseError> {
        
        let mut job_id = 1;
        let mut commandmap = Vec::<Vec<Token>>::new();
        let mut buffer: Vec<Token> = Vec::new();

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
                            redirects.push([String::from(dest.clone()), 
                                            String::from("<"), 
                                            String::from("0")]);
                            rd_from_stdin = false;
                            continue;
                        }
                        Token::DQuote(mut dest) => {
                            expand_variables(shell, &mut dest);
                            redirects.push([String::from(dest.clone()), 
                                            String::from("<"), 
                                            String::from("0")]);
                            rd_from_stdin = false;
                            continue;
                        }
                        Token::SQuote(dest) => {
                            redirects.push([String::from(dest.clone()), 
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
                    Token::Word(string) => {
                        if cmd_idx == 0 {
                            if string == "for" {

                            }
                        }
                        buffer.push((Quote::NQuote, string));
                    }
                    Token::DQuote(string) => {
                        buffer.push((Quote::DQuote, string));
                    }
                    Token::SQuote(string) => {
                        buffer.push((Quote::SQuote, string));
                    }
                    Token::BQuote(string) => {
                        match substitute_commands(shell, string) {
                            Ok(string) => {
                                buffer.push((Quote::NQuote, string));
                            }
                            Err(e) => {
                                eprintln!("{}", e);
                                return Err(ParseError::GenericError);
                            }
                        }
                    }
                    Token::SqBrkt(string) => {
                        buffer.push((Quote::SqBrkt, string));
                    }
                    Token::Brace(string) => {
                        let expanded = expand_braces(string);
                        buffer.extend(expanded);
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
        Ok(jobs)
    }
}
