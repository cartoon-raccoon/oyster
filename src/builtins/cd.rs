use std::env;
use crate::types::Cmd;
use crate::shell::Shell;

pub fn run(shell: &mut Shell, cmd: Cmd) -> i32 {
    if cmd.args.len() > 2 {
        eprintln!("cd: too many arguments");
        return 1;
    } else if cmd.args.len() == 1 {
        eprintln!("cd: not enougb arguments");
        return 1;
    }
    if cmd.redirects.len() > 0 {
        eprintln!("cd: redirects not accepted")
    }
    let pwd = env::var("PWD").unwrap_or(String::new());
    shell.set_prev_dir(pwd);
    match env::set_current_dir(cmd.args[1].clone()) {
        Ok(()) => {
            env::set_var("PWD", cmd.args[1].clone());
            shell.set_current_dir(cmd.args[1].clone());
            return 0;
        }
        Err(e) => {
            eprintln!("cd: {}",e );
            return 2;
        }
    }
}