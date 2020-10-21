use crate::shell::{self, Shell};
use crate::types::Cmd;

pub fn run(shell: &mut Shell, mut cmd: Cmd) -> i32 {
    if cmd.args.len() > 2 {
        eprintln!("set: too many arguments");
        return 2;
    }
    if shell::assign_variables(shell, &mut cmd.args[1]) {
        return 0;
    } else {
        eprintln!("set: error parsing arguments");
        return 1;
    }
}