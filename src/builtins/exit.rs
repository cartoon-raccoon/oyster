use crate::types::Cmd;
use crate::shell::Shell;

use std::process;

pub fn run(shell: &mut Shell, cmd: Cmd) -> i32 {
    if cmd.args.len() > 2 {
        eprintln!("oyster: too many arguments");
        return 1;
    }
    if cmd.args.len() == 2 {
        let code = cmd.args[1].clone();
        match code.parse::<i32>() {
            Ok(i) => {
                process::exit(i);
            }
            Err(_) => {
                eprintln!("oyster: numeric codes only.");
                process::exit(255);
            }
        }
    }
    for (_i, job) in shell.jobs.iter() {
        if !job.firstcmd.starts_with("nohup ") {
            eprintln!("oyster: there are still jobs running!");
            eprintln!("use exit 1 to force exit.");
            return 0;
        }
    }
    process::exit(0);
}