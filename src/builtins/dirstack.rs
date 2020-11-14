use std::path::PathBuf;

use crate::shell::Shell;
use crate::types::{
    Cmd,
    ShellError
};

impl Shell {
    pub fn push_dirstack(
        &mut self, 
        path: Option<String>, //either path or idx is Some
        idx: Option<usize>, 
        from: From) -> Result<PathBuf, ShellError> {
        if let Some(path) = path {
            let to_push = PathBuf::from(&path);
            if to_push.exists() {
                self.dirstack.push(PathBuf::from(to_push));
                return Ok(PathBuf::from(path));
            } else {
                return Err(ShellError::from("path does not exist"))
            }
        }
        if let Some(idx) = idx {
            if idx + 1 > self.dirstack.len() {
                return Err(ShellError::from("stack not large enough"))
            }
            let idx = match from {
                From::Left => {
                    idx
                }
                From::Right => {
                    self.dirstack.len() - idx
                }
            };
            let to_push = self.dirstack.remove(idx);
            self.dirstack.push(to_push.clone());
            return Ok(to_push)
        } else {
            unreachable!()
        }
    }
    pub fn pop_dirstack(&mut self, idx: Option<usize>, from: From) 
    -> Option<PathBuf> {
        if let Some(idx) = idx {
            if idx + 1 > self.dirstack.len() {
                return None
            }
            let idx = match from {
                From::Left => {
                    idx
                }
                From::Right => {
                    self.dirstack.len() - idx
                }
            };
            Some(self.dirstack.remove(idx))
        } else {
            self.dirstack.pop()
        }
    }
}

pub fn dirs(shell: &mut Shell, _cmd: Cmd) -> i32 {
    for path in shell.dirstack.iter().rev() {
        println!("{}", path.to_str().unwrap())
    }
    0
}

pub fn pushd(shell: &mut Shell, cmd: Cmd) -> i32 {
    if cmd.args.len() > 3 {
        eprintln!("pushd: too many arguments");
        return 1
    }
    if cmd.args.len() == 3 {
        if cmd.args[1] != "-n" {
            eprintln!("pushd: invalid switch {}", cmd.args[1]);
            return 1
        }
        if cmd.args[2].starts_with("+") {
            let result = push_dirstack(shell, &cmd.args[2][1..], From::Left, false);
            return result
        } else if cmd.args[2].starts_with("-") {
            let result = push_dirstack(shell, &cmd.args[2][1..], From::Right, false);
            return result
        } else {
            let result = push_dirstack(shell, &cmd.args[2], From::Left, false);
            return result
        }
    } else if cmd.args.len() == 2 {
        if cmd.args[1].starts_with("+") {
            let result = push_dirstack(shell, &cmd.args[1][1..], From::Left, true);
            return result
        } else if cmd.args[1].starts_with("-") {
            let result = push_dirstack(shell, &cmd.args[1][1..], From::Right, true);
            return result
        } else {
            let result = push_dirstack(shell, &cmd.args[1], From::Left, true);
            return result
        }
    } else {
        unimplemented!()
    }
}

pub fn popd(shell: &mut Shell, cmd: Cmd) -> i32 {
    if cmd.args.len() > 3 {
        eprintln!("popd: too many arguments");
        return 1
    }
    if cmd.args.len() == 3 {
        if cmd.args[1] != "-n" {
            eprintln!("popd: invalid switch {}", cmd.args[1]);
            return 1
        }
        if cmd.args[2].starts_with("+") {
            let result = pop_dirstack(shell, &cmd.args[2][1..], From::Left, false);
            return result
        } else if cmd.args[2].starts_with("-") {
            let result = pop_dirstack(shell, &cmd.args[2][1..], From::Right, false);
            return result
        } else {
            eprintln!("popd: invalid input {}", cmd.args[2]);
            return 1
        }
    } else if cmd.args.len() == 2 {
        if cmd.args[1].starts_with("+") {
            let result = pop_dirstack(shell, &cmd.args[2][1..], From::Left, true);
            return result
        } else if cmd.args[1].starts_with("-") {
            let result = pop_dirstack(shell, &cmd.args[2][1..], From::Right, true);
            return result
        } else {
            eprintln!("popd: invalid input {}", cmd.args[1]);
            return 1
        }
    } else {
        if let Some(path) = shell.dirstack.pop() {
            match shell.change_dir(path) {
                Ok(_) => {return 0},
                Err(_) => {return 3}
            }
        } else {
            eprintln!("popd: directory stack is empty");
            return 2
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum From{
    Left,
    Right,
}

fn push_dirstack(shell: &mut Shell, input: &str, from: From, cd: bool) -> i32 {
    if let Ok(idx) = input.parse::<usize>() {
        match shell.push_dirstack(None, Some(idx), from) {
            Ok(path) => {
                if cd {
                    match shell.change_dir(path) {
                        Ok(_) => {return 0}
                        Err(_) => {
                            eprintln!("pushd: cd error");
                            return 3
                        }
                    }
                } else {
                    return 0
                }
            }
            Err(e) => {
                eprintln!("pushd: {}", e);
                return 2
            }
        }
    } else {
        match shell.push_dirstack(Some(input.to_string()), None, from) {
            Ok(path) => {
                if cd {
                    match shell.change_dir(path) {
                        Ok(_) => {return 0}
                        Err(_) => {
                            eprintln!("pushd: cd error");
                            return 3
                        }
                    }
                } else {
                    return 0
                }
            }
            Err(e) => {
                eprintln!("pushd: {}", e);
                return 2
            }
        }
    }
}

fn pop_dirstack(shell: &mut Shell, input: &str, from: From, cd: bool) -> i32 {
    if let Ok(idx) = input.parse::<usize>() {
        if let Some(path) = shell.pop_dirstack(Some(idx), from) {
            if cd {
                match shell.change_dir(path) {
                    Ok(_) => {return 0}
                    Err(_) => {
                        eprintln!("popd: cd error");
                        return 3
                    }
                }
            } else {
                return 0
            }
        } else {
            eprintln!("popd: stack is smaller than {}", idx);
            return 2
        }
    } else {
        eprintln!("popd: invalid argument {}", input);
        return 2
    }
}