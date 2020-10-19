mod parser;
mod execute;
mod types;
mod shell;
mod core;
mod builtins;

use std::error::Error;
use std::process;
use std::io::{self, Write};
use std::env;

use nix::sys::signal::{signal, Signal, SigHandler,};

use parser::Lexer;
use parser::TokenizeResult::*;
use execute::*;
use shell::Shell;

const PROMPT: &'static str = ">>>>>";

fn main() -> Result<(), Box<dyn Error>> {
    //TODO: Check if login shell
    unsafe {
        signal(Signal::SIGINT, SigHandler::SigIgn)?;
        signal(Signal::SIGQUIT, SigHandler::SigIgn)?;
    }
    let mut shell = Shell::new();

    let args: Vec<String> = env::args().collect();
    if args.len() > 0 && args[0].starts_with('-') {
        shell.is_login = true;
    }
    
    loop {
        print!("{} ", PROMPT); io::stdout().flush().unwrap();
        let mut buffer = String::new();

        loop {
            match io::stdin().read_line(&mut buffer) {
                Ok(_) => {} //returns bytes read, use somewhere?
                Err(_) => {eprintln!("Shell: Could not read input")}
            }
            if buffer.trim() == "exit" {
                process::exit(0);
            }
            match Lexer::tokenize(buffer.as_str().trim()) {
                UnmatchedDQuote => {
                    print!("dquote> "); io::stdout().flush().unwrap();
                }
                UnmatchedSQuote => {
                    print!("squote> "); io::stdout().flush().unwrap();
                }
                EndsOnAnd => {
                    print!("cmdand> "); io::stdout().flush().unwrap();
                }
                EndsOnOr => {
                    print!("cmdor> "); io::stdout().flush().unwrap();
                }
                EndsOnPipe => {
                    print!("pipe> "); io::stdout().flush().unwrap();
                }
                EmptyCommand => {
                    break;
                }
                Good(parsedtokens) => {
                    match execute_jobs(&mut shell, parsedtokens) {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("{}", e.to_string());
                        }
                    }
                    break;
                }
            }
        }
    }
}
