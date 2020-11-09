use crate::shell::Shell;
use crate::types::{Cmd, Variable as Var};

pub fn run(shell: &mut Shell, cmd: Cmd) -> i32 {
    // let <type> <name> = <value>
    if cmd.args.len() == 5 { //both type specification and equals
        if cmd.args[1] == "str" {
            shell.add_variable(&cmd.args[2], Var::Str(cmd.args[4].clone()));
        } else if cmd.args[1] == "int" {
            if let Ok(int) = cmd.args[4].parse::<i32>() {
                shell.add_variable(&cmd.args[2], Var::Int(int));
            } else {
                eprintln!("let: cannot parse '{}' as int", cmd.args[4]);
                return 2;
            }
        } else if cmd.args[1] == "flt" {
            if let Ok(flt) = cmd.args[4].parse::<f32>() {
                shell.add_variable(&cmd.args[2], Var::Flt(flt));
            } else {
                eprintln!("let: cannot {} as flt", cmd.args[4]);
                return 2;
            }
        } else {
            eprintln!("let: invalid type specification")
        }
    } else if cmd.args.len() == 4 {
        // let <name> = <value> (type inference)
        if cmd.args[2] != "=" {
            eprintln!("let: invalid syntax");
            return 1;
        }
        shell.add_variable(&cmd.args[1], Var::from(&cmd.args[3]))
    } else {
        eprintln!("let: not enough arguments");
        return 1;
    }
    0
} 