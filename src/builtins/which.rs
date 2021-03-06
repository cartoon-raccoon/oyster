use crate::types::Cmd;
use crate::shell::Shell;

pub fn run(shell: &mut Shell, mut cmd: Cmd) -> i32 {
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
            name @ "show" |
            name @ "dirs" |
            name @ "pushd" |
            name @ "popd" |
            name @ "exit" => {
                println!("{}: built in shell command", name);
            }
            n@ "for" |
            n@ "while" |
            n@ "if" |
            n@ "else" |
            n@ "elif" |
            n@ "end" |
            n@ "func" |
            n@ "endfn" |
            n@ "done" => {
                println!("{}: shell reserved word", n);
            }
            _ => {
                match shell.search_in_path(&arg) {
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