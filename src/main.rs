mod parser;
mod execute;
mod types;
mod shell;

use std::error::Error;
use std::process;
use std::io::{self, Write};

use nix::sys::signal::{signal, Signal, SigHandler,};

use parser::Lexer;
use parser::TokenizeResult::*;

const PROMPT: &'static str = ">>>>>";

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        signal(Signal::SIGINT, SigHandler::SigIgn)?;
        signal(Signal::SIGQUIT, SigHandler::SigIgn)?;
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
                    println!("{:#?}", Lexer::parse_tokens(parsedtokens));
                    //execution happens here
                    break;
                }
            }
        }
    }
}
