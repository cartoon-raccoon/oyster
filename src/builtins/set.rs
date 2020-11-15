use regex::Regex;

use crate::shell::Shell;
use crate::types::{Cmd, Variable as Var};

pub fn run(shell: &mut Shell, cmd: Cmd) -> i32 {
    let re = Regex::new(r"[a-zA-Z0-9_]+").unwrap();
    // let <type> <name> = <value>
    if cmd.args.len() == 5 { //both type specification and equals
        if !re.is_match(&cmd.args[2]) {
            eprintln!("let: use alphanumeric characters and underscores only");
            return 3
        }
        if cmd.args[1] == "str" {
            shell.add_variable(&cmd.args[2], Var::Str(cmd.args[4].clone()));
        } else if cmd.args[1] == "int" {
            if let Ok(int) = cmd.args[4].parse::<i64>() {
                shell.add_variable(&cmd.args[2], Var::Int(int));
            } else {
                eprintln!("let: cannot parse '{}' as int", cmd.args[4]);
                return 2;
            }
        } else if cmd.args[1] == "flt" {
            if let Ok(flt) = cmd.args[4].parse::<f64>() {
                shell.add_variable(&cmd.args[2], Var::Flt(flt));
            } else {
                eprintln!("let: cannot parse '{}' as flt", cmd.args[4]);
                return 2;
            }
        } else if cmd.args[1] == "arr" {
            let mut input = cmd.args[4].clone();
            input.pop();
            shell.add_variable(&cmd.args[2], Var::Arr(split_arr(&input[1..])));
        } else {
            eprintln!("let: invalid type specification")
        }
    } else if cmd.args.len() == 4 {
        // let <name> = <value> (type inference)
        if !re.is_match(&cmd.args[1]) {
            eprintln!("let: use alphanumeric characters and underscores only");
            return 3
        }
        if cmd.args[2] != "=" {
            eprintln!("let: invalid syntax");
            return 1;
        }
        if cmd.args[3].starts_with("[") && cmd.args[3].ends_with("]") {
            let mut input = cmd.args[3].clone();
            input.pop();
            shell.add_variable(&cmd.args[1], Var::Arr(split_arr(&input[1..])));
        } else {
            shell.add_variable(&cmd.args[1], Var::from(&cmd.args[3]))
        }
    } else {
        eprintln!("let: not enough arguments");
        return 1;
    }
    0
}

fn split_arr(input: &str) -> Vec<Var> {
    input.split(",").map(|string| {
        Var::from(string.trim())
    }).collect()
}