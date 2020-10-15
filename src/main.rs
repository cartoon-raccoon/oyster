extern crate regex;
extern crate nix;

mod parser;
mod execute;
mod types;

use std::error::Error;
use std::process;
use std::io::{self, Write};

use nix::sys::signal::{signal, Signal, SigHandler,};

use parser::Lexer;
use parser::ParseResult;

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
            match Lexer::parse(buffer.as_str().trim()) {
                ParseResult::UnmatchedDQuote => {
                    println!("{:?}", buffer);
                    buffer.pop();
                    print!("dquote> "); io::stdout().flush().unwrap();
                    println!("{:?}", buffer);
                }
                ParseResult::UnmatchedSQuote => {
                    buffer.pop();
                    print!("squote> "); io::stdout().flush().unwrap();
                }
                ParseResult::EmptyCmd => {
                    break;
                }
                ParseResult::Good(parsedtokens) => {
                    println!("{:?}", buffer);
                    println!("{:?}", parsedtokens);
                    //execution happens here
                    break;
                }
            }
        }
    }
}
