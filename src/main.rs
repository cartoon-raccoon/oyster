mod parser;

use std::io::{self, Write};

const PROMPT: &'static str = ">>>>>";

fn main() {
    loop {
        print!("{} ", PROMPT); io::stdout().flush().unwrap();
        let mut buffer = String::new();
        match io::stdin().read_line(&mut buffer) {
            Ok(_) => {} //returns bytes read, use somewhere?
            Err(_) => {eprintln!("Shell: Could not read input")}
        }
        println!("{}", buffer);
    }
}
