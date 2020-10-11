//use std::borrow::Borrow;

#[derive(Debug, Clone)]
pub enum Token<'a> {
    Command(Cmd<'a>),
    Builtin(Builtin<'a>),
    Word(Vec<&'a str>),
    Pipe,
    And,
    Or,
    Redirect,
    RDAppend,
    Background,
}

#[derive(Debug, Clone)]
pub enum Builtin<'a> {
    Cd(Vec<&'a str>),
    Which(Vec<&'a str>),
    Eval(&'a str),
    Source(&'a str), //use PathBuf instead?
    Echo(Vec<&'a str>),
    Alias(&'a str),
    Read,
    Kill(Vec<&'a str>),
    Exit,

}

#[derive(Debug, Clone)]
pub struct Cmd<'a> {
    pub cmd: &'a str,
    pub args: Vec<&'a str>
}

// impl<'a, B: Borrow<Cmd<'a>> + 'a> From<Vec<&'a str>> for B {
//     fn from(words: Vec<&'a str>) -> Cmd<'a> {
//         Cmd {
//             cmd: words[0],
//             args: words[0..].to_vec()
//         }
//     }
// }

pub struct Lexer {

}


impl Lexer { //TODO: Implement quotation delimiting and escaping with backslashes
    pub fn parse<'a>(line: &'a str) -> Option<Vec<Token>> {
        let mut splitcmd = line.split_whitespace().peekable();

        let mut elements = Vec::<Token>::new();
        let mut build = Vec::<&str>::new();

        while let Some(elem) = splitcmd.next() {
            match match_token(elem) {
                Some(token) => {elements.push(token)}
                None => {
                    build.push(elem);

                    //lookahead to determine whether the next token is a symbol
                    if let Some(_) = match_token(splitcmd.peek().unwrap_or(&"")) {
                        elements.push(Token::Word(build.clone()));
                        build.clear();
                    } else if let None = splitcmd.peek() {
                        elements.push(Token::Word(build.clone()));
                        build.clear();
                    }
                }
            }
        }

        for element in elements.iter_mut() {
            if let Token::Word(words) = element {
                match words[0] { //eval is currently unsupported
                    "cd" => {*element = Token::Builtin(Builtin::Cd(words[1..].to_vec()));}
                    "which" => {*element = Token::Builtin(Builtin::Which(words[1..].to_vec()));}
                    "echo" => {*element = Token::Builtin(Builtin::Echo(words[1..].to_vec()));}
                    "alias" => {*element = Token::Builtin(Builtin::Alias(words[1]));}
                    "source" => {*element = Token::Builtin(Builtin::Source(words[1]));}
                    "eval" => {*element = Token::Builtin(Builtin::Eval(words[1]));}
                    "kill" => {*element = Token::Builtin(Builtin::Kill(words[1..].to_vec()));}
                    "read" => {*element = Token::Builtin(Builtin::Read);}
                    "exit" => {*element = Token::Builtin(Builtin::Exit);}
                    _ => { *element = Token::Command(
                        Cmd {cmd: words[0], args: words[1..].to_vec()}
                    );}
                }
            }
        }
        
        if elements.len() > 0 {Some(elements)} else {None}
    }
}

fn match_token(token: &str) -> Option<Token> {
    match token {
        "|" => Some(Token::Pipe),
        "&&" => Some(Token::And),
        "||" => Some(Token::Or),
        ">>" => Some(Token::RDAppend),
        ">" => Some(Token::Redirect),
        "&" => Some(Token::Background),
        _ => None,
    }
}