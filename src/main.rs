mod parser;
mod execute;
mod types;
mod shell;
mod core;
mod jobc;
mod prompt;
mod builtins;
mod scripting;

#[macro_use]
extern crate lazy_static;

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
use types::{
    Variable,
    TokenizeResult,
    ParseResult,
};
use execute::*;
use shell::Shell;
use scripting::execute_scriptfile;

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        signal(Signal::SIGINT, SigHandler::SigIgn)?;
        signal(Signal::SIGQUIT, SigHandler::SigIgn)?;
        signal(Signal::SIGTSTP, SigHandler::SigDfl)?;
        signal(Signal::SIGCHLD, SigHandler::Handler(sigchld_handler))?;
    }

    let lr = Interface::new("oyster")?;
    let mut shell = Shell::with_config("testconfig");
    let mut last_status: i32 = 0;

    let args: Vec<String> = env::args().collect();
    if args.len() > 0 && args[0].starts_with('-') {
        shell.is_login = true;
    }
    if args.len() >= 3 {
        let mut counter = 0;
        for arg in &args[2..] {
            let varname = format!("{}", counter);
            shell.add_variable(&varname, Variable::from(arg));
            counter += 1;
        }
    }
    if args.len() > 1 {
        if args[1] == "-i" {

        } else {
            let status = match execute_scriptfile(&mut shell, &args[1]) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(10);
                }
            };
            process::exit(status)
        }
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
                        TokenizeResult::EmptyCommand => {
                            buffer.clear();
                            last_status = 0;
                            break;
                        }
                        TokenizeResult::Good(parsedtokens) => {
                            let jobs = match Lexer::parse_tokens(&mut shell, parsedtokens) {
                                Ok(result) => result,
                                Err(e) => {
                                    eprintln!("{}", e);
                                    last_status = 2;
                                    buffer.clear();
                                    break
                                }
                            };
                            match jobs {
                                ParseResult::Good(jobs) => {
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
                                n@ _ => {
                                    print!("{} ", n); io::stdout().flush().unwrap();
                                    match io::stdin().read_line(&mut buffer) {
                                        Ok(_) => {},
                                        Err(_) => {
                                            eprintln!("oyster: error reading to line");
                                        }
                                    }
                                }
                            }
                        }
                        n@ _ => {
                            print!("{} ", n); io::stdout().flush().unwrap();
                            match io::stdin().read_line(&mut buffer) {
                                Ok(_) => {},
                                Err(_) => {
                                    eprintln!("oyster: error reading to line");
                                }
                            }
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
