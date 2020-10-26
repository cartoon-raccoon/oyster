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
use rustyline::Editor;

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

    let mut linereader = Editor::<()>::new();
    let mut shell = Shell::new();
    let mut last_status: i32 = 0;

    let args: Vec<String> = env::args().collect();
    if args.len() > 0 && args[0].starts_with('-') {
        shell.is_login = true;
    }
    
    loop {
        jobc::try_wait_bg_jobs(&mut shell);
        let prompt = prompt::render_prompt(last_status);
        let mut buffer = String::new();

        match linereader.readline(&prompt) {
            Ok(line) => {
                buffer.push_str(&line);
            } //returns bytes read, use somewhere?
            Err(_) => {
                last_status = 1;
                continue;
            }
        }
        loop {
            match Lexer::tokenize(buffer.trim().to_string()) {
                Ok(result) => {
                    match result {
                        n@ UnmatchedDQuote | n@ UnmatchedSQuote | n@ UnmatchedBQuote |
                        n@ EndsOnAnd | n@ EndsOnOr | n@ EndsOnPipe => {
                            print!("{} ", n); io::stdout().flush().unwrap();
                            match io::stdin().read_line(&mut buffer) {
                                Ok(_) => {},
                                Err(_) => {
                                    eprintln!("oyster: error reading to line");
                                }
                            }
                        }
                        EmptyCommand => {
                            buffer.clear();
                            break;
                        }
                        Good(parsedtokens) => {
                            match execute_jobs(&mut shell, parsedtokens, false) {
                                Ok(result) => {
                                    last_status = result.0;
                                }
                                Err(e) => {
                                    eprintln!("{}", e.to_string());
                                    last_status = 10;
                                }
                            }
                            buffer.clear();
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
