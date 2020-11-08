use crate::shell::Shell;
use crate::types::Cmd;

pub fn run(shell: &mut Shell, cmd: Cmd) -> i32 {
    let key_value: Vec<&str>;
    if cmd.args.len() == 4 {
        if cmd.args[2] != "=" {
            eprintln!("oyster: invalid syntax");
            return 2;
        }
        key_value = vec![&cmd.args[1], &cmd.args[3]];
    } else if cmd.args.len() == 3 {
        key_value = vec![&cmd.args[1], &cmd.args[2]];
    } else {
        eprintln!("oyster: too many arguments");
        return 1;
    }
    if key_value.len() != 2 {
        eprintln!("oyster: bad assignment for `{}`", key_value[0]);
        return 1;
    }
    shell.add_variable(key_value[0], key_value[1]);
    0
}