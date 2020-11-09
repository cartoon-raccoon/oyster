use crate::shell::Shell;
use crate::types::{
    Cmd,
};

pub fn run(shell: &mut Shell, cmd: Cmd) -> i32 {
    if cmd.args.len() < 2 {
        eprintln!("show: no arguments given");
        return 1;
    }
    if cmd.args[1].starts_with("-") {
        if cmd.args.len() != 3 {
            eprintln!("show: incorrect number of arguments");
            return 1;
        }
        let to_find: &str = &cmd.args[2];
        match cmd.args[1].as_str() {
            "-f" => {
                if let Some(func) = shell.funcs.get(to_find) {
                    func.print();
                } else {
                    eprintln!("show: could not find function {} in shell", to_find);
                    return 2;
                }
            }
            "-v" => {
                if let Some(var) = shell.get_variable(to_find) {
                    var.print();
                } else {
                    eprintln!("show: could not find variable {} in shell", to_find);
                    return 2;
                }
            }
            "-a" => {
                if let Some(alias) = shell.get_alias(to_find) {
                    println!("{}", alias);
                } else {
                    eprintln!("show: could not find alias {} in shell", to_find)
                }
            }
            n@ _ => {
                eprintln!("show: unknown option `{}`", n)
            }
        }
    } else {
        if cmd.args.len() != 2 {
            eprintln!("show: incorrect number of arguments");
            return 1;
        }
        if let Some(func) = shell.funcs.get(&cmd.args[1]) {
            func.print();
        } else if let Some(var) = shell.get_variable(&cmd.args[1]) {
            var.print();
        } else if let Some(alias) = shell.get_alias(&cmd.args[1]) {
            println!("{}", alias);
        } else {
            eprintln!(
                "show: could not find matching variable, function or alias in shell"
            );
            return 2;
        }
    }
    0
}