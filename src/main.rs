mod parser;
mod execute;
mod types;
mod shell;
mod core;
mod jobc;
mod prompt;
mod builtins;

use std::error::Error;
use std::io::{self, Write};
use std::env;

use nix::sys::signal::{signal, Signal, SigHandler,};

use parser::Lexer;
use types::TokenizeResult::*;
use execute::*;
use shell::Shell;

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        signal(Signal::SIGINT, SigHandler::SigIgn)?;
        signal(Signal::SIGQUIT, SigHandler::SigIgn)?;
        signal(Signal::SIGTSTP, SigHandler::SigDfl)?;
    }
    let mut shell = Shell::new();
    let mut last_status: i32 = 0;

    let args: Vec<String> = env::args().collect();
    if args.len() > 0 && args[0].starts_with('-') {
        shell.is_login = true;
    }
    
    loop {
        jobc::try_wait_bg_jobs(&mut shell);
        let prompt = prompt::get_prompt(last_status);
        print!("{} ", prompt); io::stdout().flush().unwrap();
        let mut buffer = String::new();

        loop {
            match io::stdin().read_line(&mut buffer) {
                Ok(_) => {} //returns bytes read, use somewhere?
                Err(_) => {eprintln!("Shell: Could not read input")}
            }
            match Lexer::tokenize(&mut shell, buffer.trim().to_string(), false) {
                Ok(result) => {
                    match result {
                        n@ UnmatchedDQuote | n@ UnmatchedSQuote |
                        n@ EndsOnAnd | n@ EndsOnOr | n@ EndsOnPipe => {
                            print!("{} ", n); io::stdout().flush().unwrap();
                        }
                        EmptyCommand => {
                            break;
                        }
                        Good(parsedtokens) => {
                            match execute_jobs(&mut shell, parsedtokens, false) {
                                Ok(result) => {
                                    last_status = result.0;
                                }
                                Err(e) => {
                                    eprintln!("{}", e.to_string());
                                }
                            }
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    break;
                }
            }
        }
    }
}
