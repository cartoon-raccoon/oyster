use crate::types::{
    Redirect,
    ExecCondition,
    Token,
    Cmd,
    Job,
    JobStatus,
    ParseError,
    ParseResult,
};

pub enum TokenizeResult {
    UnmatchedDQuote,
    UnmatchedSQuote,
    EndsOnOr,
    EndsOnAnd,
    EndsOnPipe,
    EmptyCommand,
    Good(Vec<Token>),
}

pub struct Lexer;

impl Lexer {

    /// Tokenizes the &str into a Vec of tokens
    pub fn tokenize<'a>(line: &'a str) -> TokenizeResult {
        let mut line_iter = line.chars().peekable();

        //Accumulators
        let mut tokenvec = Vec::<Token>::new();
        let mut word = String::new();

        //Trackers
        let mut in_dquote = false;
        let mut in_squote = false;
        let mut escaped = false;
        let mut ignore_next = false;
        let mut prev_char = None;

        let push = |elements: &mut Vec<Token>, 
                    charvec: &mut String,
                    character: char,
                    in_double_quotes: bool,
                    in_single_quotes: bool,| {
            if !in_double_quotes && !in_single_quotes {
                elements.push(Token::Word(charvec.clone()));
                charvec.clear();
            } else {
                charvec.push(character)
            }
        };
        
        //* Phase 1: Tokenisation
        while let Some(c) = line_iter.next() {
            // println!("========================");
            // println!("{:?}", c);
            // println!("{:?}", prev_char);
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
                '|' if line_iter.peek() == Some(&'|') && !escaped => {
                    push(&mut tokenvec, &mut word, c, in_dquote, in_squote);
                    if !in_dquote && !in_squote {
                        tokenvec.push(Token::Or);
                        ignore_next = true;
                    }
                },
                '|' if line_iter.peek() == Some(&'&') && !escaped => {
                    push(&mut tokenvec, &mut word, c, in_dquote, in_squote);
                    if !in_dquote && !in_squote {
                        tokenvec.push(Token::Pipe2);
                        ignore_next = true;
                    }
                },
                '|' if line_iter.peek() != Some(&'|') && !escaped => {
                    push(&mut tokenvec, &mut word, c, in_dquote, in_squote);
                    if !in_dquote && !in_squote {
                        tokenvec.push(Token::Pipe);
                        ignore_next = false;
                    }
                },
                '&' if line_iter.peek() == Some(&'&') && !escaped => {
                    push(&mut tokenvec, &mut word, c, in_dquote, in_squote);
                    if !in_dquote && !in_squote {
                        tokenvec.push(Token::And);
                        ignore_next = true;
                    }
                },
                '&' if line_iter.peek() == Some(&'>') && !escaped => {
                    push(&mut tokenvec, &mut word, c, in_dquote, in_squote);
                    if !in_dquote && !in_squote {
                        tokenvec.push(Token::RDStdOutErr);
                        ignore_next = true;
                    }
                }
                '&' if line_iter.peek() != Some(&'&') && !escaped => {
                    push(&mut tokenvec, &mut word, c, in_dquote, in_squote);
                    if !in_dquote && !in_squote {
                        if prev_char == Some('>') {
                            tokenvec.push(Token::FileMarker);
                        } else {
                            tokenvec.push(Token::Background);
                        }
                        ignore_next = false;
                    }
                },
                '>' if line_iter.peek() == Some(&'>') && !escaped => {
                    push(&mut tokenvec, &mut word, c, in_dquote, in_squote);
                    if !in_dquote && !in_squote {
                        tokenvec.push(Token::RDAppend);
                        ignore_next = true;
                    }
                },
                '>' if line_iter.peek() == Some(&'&') && !escaped => {
                    push(&mut tokenvec, &mut word, c, in_dquote, in_squote);
                    if !in_dquote && !in_squote {
                        tokenvec.push(Token::RDFileDesc);
                        ignore_next = true;
                    }
                }
                '>' if line_iter.peek() != Some(&'>') && !escaped => {
                    push(&mut tokenvec, &mut word, c, in_dquote, in_squote);
                    if !in_dquote && !in_squote {
                        tokenvec.push(Token::Redirect);
                        ignore_next = false;
                    }
                },
                ';' if !escaped => {
                    push(&mut tokenvec, &mut word, c, in_dquote, in_squote);
                    if !in_dquote && !in_squote {
                        tokenvec.push(Token::Consec);
                        ignore_next = false;
                    }
                }
                '\'' if !escaped => {
                    ignore_next = false;
                    if in_squote && !in_dquote  {
                        in_squote = false;
                        tokenvec.push(Token::SQuote(word.clone()));
                        word.clear();
                    } else if in_dquote {
                        word.push(c);
                    } else {
                        in_squote = true;
                    }
                }
                '"' if !escaped => {
                    ignore_next = false;
                    if in_dquote && !in_squote {
                        in_dquote = false;
                        tokenvec.push(Token::DQuote(word.clone()));
                        word.clear();
                    } else if in_squote {
                        word.push(c);
                    } else {
                        in_dquote = true;
                    }
                }
                '\\' if !escaped => {
                    if in_squote {
                        word.push(c);
                    } else {
                        escaped = true;
                        prev_char = Some(c);
                        continue;
                    }
                }
                ' ' => {
                    ignore_next = false;
                    if prev_char != Some(' ') {
                        if !in_dquote && !in_squote && !escaped {
                            tokenvec.push(Token::Word(word.clone())); 
                            word.clear();
                        } else {
                        word.push(c);
                        }   
                    }
                    if escaped {
                        escaped = false;
                    }
                }
                _ => {
                    ignore_next = false;
                    escaped = false;
                    word.push(c);
                }
            }
            prev_char = Some(c)
        }

        tokenvec.push(Token::Word(word));

        // filtering empty words
        let tokenvec: Vec<Token> = tokenvec.into_iter()
            .filter(|token| {
                if let Token::Word(word) = token {
                    if word == "" {
                        return false
                    } else {
                        return true
                    }
                }
                true
            }).collect();
        
        if in_dquote {
            return TokenizeResult::UnmatchedDQuote;
        } else if in_squote {
            return TokenizeResult::UnmatchedSQuote;
        } else {
            match tokenvec.last() {
                Some(token) => {
                    match *token {
                        Token::Or => {
                            return TokenizeResult::EndsOnOr;
                        }
                        Token::And => {
                            return TokenizeResult::EndsOnAnd;
                        }
                        Token::Pipe | Token::Pipe2 => {
                            return TokenizeResult::EndsOnPipe;
                        }
                        _ => {}
                    }
                }
                None => {
                    return TokenizeResult::EmptyCommand;
                }
            }
        }

        if tokenvec.len() > 0 {
            TokenizeResult::Good(tokenvec)
        } else {
            TokenizeResult::EmptyCommand
        }
    }

    /// Splits command and parses special characters
    
    //TODO: Add variable expansion and command expansion
    pub fn parse_tokens(tokens: Vec<Token>) -> ParseResult {

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
        commandmap.push(buffer);

        //building job set
        let mut jobs = Vec::<Job>::new();

        for tokengrp in commandmap.clone() {

            //* trackers
            let mut all_to_filename = false;
            let mut rd_to_filename = false;
            let mut rd_to_filedesc = false;

            //* accumulators
            let mut buffer = Vec::<String>::new();
            let mut redirect = [String::new(), String::new(), String::new()];
            let mut redirects = Vec::<[String; 3]>::new();

            //* building job from these
            let mut cmds = Vec::<Cmd>::new();
            let mut execif = None;
            
            if commandmap[0][0] == Token::Pipe || commandmap[0][0] == Token::Pipe2 {
                return Err(ParseError::PipeMismatch)
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
                        Token::Word(dest) | 
                        Token::DQuote(dest) | 
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
                                String::from("STDOUT")
                            } else if dest == "2" {
                                String::from("STDERR")
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
                } else if all_to_filename {
                    match token {
                        Token::Word(dest) | 
                        Token::DQuote(dest) | 
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
                }
                match token {
                    pipe @ Token::Pipe | pipe @ Token::Pipe2 => {
                        let mut final_redirects = 
                            Vec::<(String, Redirect, String)>::new();
                        for redirect in &redirects {
                            let redirecttype = 
                                if redirect[1] == ">>" {Redirect::Append}
                                else {Redirect::Override};
                            final_redirects.push((redirect[0].clone(), 
                                                  redirecttype, 
                                                  redirect[2].clone()));
                        }
                        redirects.clear();
                        if buffer.len() < 1 {
                            return Err(ParseError::EmptyCommand);
                        } else {
                            cmds.push(
                                Cmd {
                                    cmd: buffer[0].clone(),
                                    args: if buffer.len() == 1 {
                                        Vec::new()
                                    } else {
                                        buffer[1..].to_vec()
                                    },
                                    redirects: final_redirects,
                                    pipe_stderr: if pipe == Token::Pipe {false} else {true},
                                }
                            );
                            buffer.clear();
                        }
                    }
                    Token::Word(string) | 
                    Token::SQuote(string) | 
                    Token::DQuote(string)=> {
                        //TODO: Perform expansion here
                        buffer.push(string);
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
                            if fd == "2" || fd == "1" {
                                //origin is file descriptor
                                redirect[0] = fd;
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
                    //* if matching on these, they must be the last item
                    Token::And => {
                        execif = Some(ExecCondition::And);
                    }
                    Token::Or => {
                        execif = Some(ExecCondition::Or);
                    }
                    Token::Consec => {
                        execif = Some(ExecCondition::Consec);
                    }
                    Token::Background => {
                        execif = Some(ExecCondition::Background);
                    }
                    _ => {
                        
                    }
                }
            }
            let mut final_redirects = 
                Vec::<(String, Redirect, String)>::new();
            for redirect in &redirects {
                let redirecttype = 
                    if redirect[1] == ">>" {Redirect::Append}
                    else {Redirect::Override};
                final_redirects.push((redirect[0].clone(), 
                                        redirecttype, 
                                        redirect[2].clone()));
            }
            redirects.clear();
            if buffer.len() < 1 {
                return Err(ParseError::EmptyCommand);
            } else {
                cmds.push(
                    Cmd {
                        cmd: buffer[0].clone(),
                        args: if buffer.len() == 1 {
                            Vec::new()
                        } else {
                            buffer[1..].to_vec()
                        },
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
                    pgid: 0,
                    status: JobStatus::InProgress,
                }
            );
            job_id += 1;
        }

        Ok(jobs)
    }
}
