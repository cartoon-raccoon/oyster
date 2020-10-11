mod parser;
mod execute;

use std::error::Error;
use std::process;
use std::io::{self, Write};

use nix::sys::signal::{signal, Signal, SigHandler,};

use parser::Lexer;

const PROMPT: &'static str = ">>>>>";

fn main() -> Result<(), Box<dyn Error>> {
    unsafe {
        signal(Signal::SIGINT, SigHandler::SigIgn)?;
        signal(Signal::SIGQUIT, SigHandler::SigIgn)?;
    }
    loop {
        print!("{} ", PROMPT); io::stdout().flush().unwrap();
        let mut buffer = String::new();
        match io::stdin().read_line(&mut buffer) {
            Ok(_) => {} //returns bytes read, use somewhere?
            Err(_) => {eprintln!("Shell: Could not read input")}
        }

        //placeholder - builtin checking is done in parse()
        if buffer.trim() == "exit" {
            process::exit(0);
        }
        if let Some(commands) = Lexer::parse(buffer.as_str()) {
            println!("{:?}", commands)
            // if execute::execute(commands) {
            //     continue;
            // } else {
            //     println!("Command exited unsuccessfully")
            // }
        }
    }
}
