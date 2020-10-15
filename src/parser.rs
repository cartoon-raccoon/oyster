use crate::types::{
    Token,
    Cmd,
    Builtin::*,
};

pub enum ParseResult {
    UnmatchedDQuote,
    UnmatchedSQuote,
    EmptyCmd,
    Good(Vec<Token>),
}

pub struct Lexer {

}


impl Lexer { //TODO: Implement quotation delimiting and escaping with backslashes
    pub fn parse<'a>(line: &'a str) -> ParseResult {
        let mut line_iter = line.chars().peekable();

        //Accumulators
        let mut tokenvec = Vec::<Token>::new();
        let mut wordvec = Vec::<String>::new();
        let mut word = String::new();

        //Trackers
        let mut doubleq = false;
        let mut ignore_next = false;
        let mut prev_char = None;

        let push = |elements: &mut Vec<Token>, 
                    build: &mut Vec<String>, 
                    charvec: &mut String,
                    character: char,
                    in_double_quotes: bool,| {
            if !in_double_quotes {
                build.push(charvec.clone());
                elements.push(Token::Word(build.clone()));
                charvec.clear();
                build.clear();
            } else {
                charvec.push(character)
            }
        };
        
        //* Phase 1: Tokenisation
        while let Some(c) = line_iter.next() {
            // println!("========================");
            // println!("{:?}", c);
            // println!("{:?}", prev_char);
            // println!("{:?}", word);
            // println!("{:?}", wordvec);
            // println!("{:?}", tokenvec);
            // println!("In Dquote: {}", doubleq);
            // println!("Ignore:    {}", doubleq);
            if ignore_next {
                ignore_next = false;
                prev_char = Some(c);
                continue;
            }
            match c {
                '|' if line_iter.peek() == Some(&'|') => {
                    push(&mut tokenvec, &mut wordvec, &mut word, c, doubleq);
                    tokenvec.push(Token::Or);
                    ignore_next = true;
                },
                '|' if line_iter.peek() == Some(&'&') => {
                    push(&mut tokenvec, &mut wordvec, &mut word, c, doubleq);
                    tokenvec.push(Token::Pipe2);
                    ignore_next = true;
                },
                '|' if line_iter.peek() != Some(&'|') => {
                    push(&mut tokenvec, &mut wordvec, &mut word, c, doubleq);
                    tokenvec.push(Token::Pipe);
                    ignore_next = false;
                },
                '&' if line_iter.peek() == Some(&'&') => {
                    push(&mut tokenvec, &mut wordvec, &mut word, c, doubleq);
                    tokenvec.push(Token::And);
                    ignore_next = true;
                },
                '&' if line_iter.peek() != Some(&'&') => {
                    push(&mut tokenvec, &mut wordvec, &mut word, c, doubleq);
                    tokenvec.push(Token::Background);
                    ignore_next = false;
                },
                '>' if line_iter.peek() != Some(&'>') => {
                    push(&mut tokenvec, &mut wordvec, &mut word, c, doubleq);
                    tokenvec.push(Token::Redirect);
                    ignore_next = false;
                },
                '>' if line_iter.peek() == Some(&'>') => {
                    push(&mut tokenvec, &mut wordvec, &mut word, c, doubleq);
                    tokenvec.push(Token::RDAppend);
                    ignore_next = true;
                },
                ';' => {
                    push(&mut tokenvec, &mut wordvec, &mut word, c, doubleq);
                    tokenvec.push(Token::Consec);
                    ignore_next = false;
                }
                '"' => {
                    ignore_next = false;
                    if doubleq {
                        doubleq = false;
                        wordvec.push(word.clone());
                        word.clear();
                    } else {
                        doubleq = true;
                    }
                }
                ' ' => {
                    ignore_next = false;
                    if prev_char != Some(' ') {
                        if !doubleq {
                            wordvec.push(word.clone()); 
                            word.clear();
                        } else {
                        word.push(c);
                        }   
                    }
                }
                _ => {
                    ignore_next = false;
                    word.push(c);
                }
            }
            prev_char = Some(c)
        }

        if doubleq {
            return ParseResult::UnmatchedDQuote;
        }

        wordvec.push(word);
        tokenvec.push(Token::Word(wordvec));

        //* Phase 2: Parsing words into commands
        let mut to_return: Vec<Token> = Vec::new();
        for token in tokenvec {
            if let Token::Word(words) = token {
                let words2: Vec<String> = words.into_iter()
                    .filter(|word| word != "")
                    .map(|word| word.trim().to_string())
                    .collect();
                if words2.is_empty() {
                    continue;
                }
                match words2[0].as_str() {
                    "cd" => {to_return.push(Token::Builtin(
                        Cd(words2.clone()))
                    );}
                    "which" => {to_return.push(Token::Builtin(
                        Which(words2.clone()))
                    );}
                    "eval" => {to_return.push(Token::Builtin(
                        Eval(words2.clone()))
                    );}
                    "source" => {to_return.push(Token::Builtin(
                        Source(words2.clone()))
                    );}
                    "echo" => {to_return.push(Token::Builtin(
                        Echo(words2.clone()))
                    );}
                    "alias" => {to_return.push(Token::Builtin(
                        Alias(words2.clone()))
                    );}
                    "unalias" => {to_return.push(Token::Builtin(
                        Unalias(words2.clone()))
                    );}
                    "kill" => {to_return.push(Token::Builtin(
                        Kill(words2[1..].to_vec()))
                    );}
                    "read" => {to_return.push(Token::Builtin(Read));}
                    "exit" => {to_return.push(Token::Builtin(Exit));}
                    _ => {
                        to_return.push(Token::Command(
                            Cmd {
                                cmd: words2[0].clone(),
                                args: words2[1..].to_vec(),
                            }
                        ));
                    }
                }
            } else {
                to_return.push(token);
            }
        }

        if to_return.len() > 0 {
            ParseResult::Good(to_return)
        } else {
            ParseResult::EmptyCmd
        }
    }
}
