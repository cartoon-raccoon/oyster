use crate::shell::Shell;
use crate::types::{
    Cmd,
    TokenizeResult,
};
use crate::parser::Lexer;

pub fn set(shell: &mut Shell, cmd: Cmd) -> i32 {
    if cmd.args.len() != 2 {
        eprintln!("oyster: bad assignment");
        return 1;
    }
    let key_value: Vec<&str> = cmd.args[1].split("=").collect();
    if key_value.len() != 2 {
        eprintln!("oyster: bad assignment");
        return 1;
    }
    match Lexer::tokenize(key_value[1]) {
        Ok(result) => {
            if let TokenizeResult::Good(tokens) = result {
                match Lexer::parse_tokens(shell, tokens) {
                    Ok(_) => {
                        shell.add_alias(key_value[0], key_value[1]);
                        return 0;
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        eprintln!("oyster: bad assignment");
                        return 1;
                    }
                }
            } else {
                eprintln!("oyster: bad assignment");
                return 1;
            }
        }
        Err(_) => {
            eprintln!("oyster: bad assignment");
            return 1;
        }
    } 
}

pub fn unset(shell: &mut Shell, cmd: Cmd) -> i32 {
    if cmd.args.len() != 2 {
        eprintln!("unalias: incorrect number of arguments");
        return 1;
    }
    if let Some(_) = shell.remove_alias(&cmd.args[1]) {
        return 0;
    } else {
        eprintln!("unalias: no such hashtable element: {}", cmd.args[1]);
        return 2;
    }
}