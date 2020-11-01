mod parser;
mod execute;
mod types;
mod shell;
mod core;
mod jobc;
mod prompt;
mod builtins;

#[allow(dead_code, unused_variables)]
mod scripting;

use std::error::Error;
use std::io::{self, Write};
use std::env;
use std::process;

use nix::sys::signal::{signal, Signal, SigHandler,};
use linefeed::{
    Interface, ReadResult,
    terminal::Signal as TSignal,
};

use parser::Lexer;
use types::TokenizeResult::*;
use execute::*;
use shell::Shell;

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        signal(Signal::SIGINT, SigHandler::SigIgn)?;
        signal(Signal::SIGQUIT, SigHandler::SigIgn)?;
        signal(Signal::SIGTSTP, SigHandler::SigDfl)?;
        signal(Signal::SIGCHLD, SigHandler::Handler(sigchld_handler))?;
    }

    let lr = Interface::new("oyster")?;
    let mut shell = Shell::new();
    let mut last_status: i32 = 0;

    let args: Vec<String> = env::args().collect();
    if args.len() > 0 && args[0].starts_with('-') {
        shell.is_login = true;
    }
    
    loop {
        jobc::try_wait_bg_jobs(&mut shell);
        let prompt = prompt::render_prompt(last_status);
        match lr.set_prompt(&prompt) {
            Ok(()) => {},
            Err(_) => {
                eprintln!("oyster: could not set prompt")
            }
        }
        let mut buffer = String::new();

        match lr.read_line() {
            Ok(ReadResult::Input(line)) => {
                buffer.push_str(&line);
                buffer.push('\n');
            }
            Ok(ReadResult::Eof) => {
                process::exit(100);
            }
            Ok(ReadResult::Signal(signal)) => {
                if let TSignal::Interrupt = signal {
                    last_status = 20;
                    continue;
                }
            }
            Err(_) => {
                last_status = 1;
                continue;
            }
        }
        loop {
            match Lexer::tokenize(&buffer) {
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
                            let jobs = match Lexer::parse_tokens(&mut shell, parsedtokens) {
                                //* this will return an enum in the future
                                //* where it matches on user's first command
                                //* i.e. on if, for, while; and waits for input
                                //* in a similar manner to tokenize
                                //* if the scripting keywords don't appear
                                //* or the scripting construct is complete,
                                //* it returns a Good enum and execution continues
                                Ok(result) => result,
                                Err(e) => {
                                    eprintln!("{}", e);
                                    last_status = 2;
                                    buffer.clear();
                                    break
                                }
                            };
                            match execute_jobs(&mut shell, jobs, false) {
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
                    last_status = 11;
                    break;
                }
            }
        }
    }
}

extern "C" fn sigchld_handler(_: libc::c_int) {
    //do something with this? idk
}
