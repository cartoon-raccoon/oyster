use std::env;
use crate::types::Cmd;
use crate::shell::Shell;

pub fn run(shell: &mut Shell, mut cmd: Cmd, implicit: bool) -> i32 {
    if cmd.args.len() > 2 {
        eprintln!("cd: too many arguments");
        return 1;
    } 
    if cmd.redirects.len() > 0 {
        eprintln!("cd: redirects not accepted")
    }
    let cd_to: String;
    if cmd.args.len() == 1 {
        if !implicit {
            cd_to = env::var("HOME").unwrap_or(String::new());
        } else {
            cd_to = cmd.args.remove(0);
        }
    } else {
        cd_to = cmd.args.remove(1);
    }
    if cd_to.is_empty() {
        eprintln!("oyster: env error, cannot set home dir");
        return 2;
    }
    let pwd = env::var("PWD").unwrap_or(String::new());
    if pwd.is_empty() {
        eprintln!("oyster: env error, cannot get pwd");
        return 2;
    }
    shell.set_prev_dir(pwd);
    match env::set_current_dir(&cd_to) {
        Ok(()) => {
            env::set_var("PWD", &cd_to);
            shell.set_current_dir(cd_to);
            return 0;
        }
        Err(e) => {
            eprintln!("cd: {}",e );
            return 2;
        }
    }
}