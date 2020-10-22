use std::env;

use crate::types::Cmd;

pub fn run(cmd: Cmd) -> i32 {
    if cmd.args.len() > 2 {
        eprintln!("export: too many arguments");
        return 1;
    }
    let key_value: Vec<&str> = cmd.args[1].split("=").collect();
    if key_value.len() != 2 {
        eprintln!("export: malformed input");
        return 1;
    }
    env::set_var(key_value[0], key_value[1]);
    0
}