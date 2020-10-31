use crate::shell::search_in_path;
use crate::types::Cmd;

pub fn run(mut cmd: Cmd) -> i32 {
    cmd.args.remove(0);
    let mut failed: i32 = 0;
    for arg in cmd.args {
        match arg.as_str() {
            name @ "cd" |
            name @ "which" |
            name @ "eval" |
            name @ "source" |
            name @ "export" |
            //name @ "echo" |
            //name @ "kill" |
            name @ "alias" |
            name @ "let" |
            name @ "exit" => {
                println!("{}: built in shell command", name);
            }
            _ => {
                match search_in_path(arg.clone()) {
                    Ok(path) => {
                        if let Some(pathname) = path.to_str() {
                            println!("{}", pathname);
                        } else {
                            eprintln!("error: path conversion failed");
                            failed += 1;
                        }
                    }
                    Err(_) => {
                        eprintln!("program {} not found in path", arg);
                        failed += 1;
                    }
                }
            }
        }
    }
    failed
}