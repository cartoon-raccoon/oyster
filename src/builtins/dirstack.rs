use std::path::PathBuf;
use std::error::Error;
use std::env;

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
                    self.dirstack.len() - 1 - idx
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
                    self.dirstack.len() - 1 - idx
                }
            };
            Some(self.dirstack.remove(idx))
        } else {
            self.dirstack.pop()
        }
    }
}

pub fn dirs(shell: &mut Shell, cmd: Cmd) -> i32 {
    struct Options {
        clear: bool,
        fullpath: bool,
        per_line: bool,
        show_idx: bool,
    }
    if cmd.args.len() > 3 {
        eprintln!("dirs: too many arguments")
    }
    let mut options = Options {
        clear: false,
        fullpath: false,
        per_line: false,
        show_idx: false,
    };
    if cmd.args.len() > 1 {
        if cmd.args[1].starts_with("-") {
            if cmd.args[1].contains("c") {
                options.clear = true;
            }
            if cmd.args[1].contains("l") {
                options.fullpath = true;
            }
            if cmd.args[1].contains("p") {
                options.per_line = true;
            }
            if cmd.args[1].contains("v") {
                options.per_line = true;
                options.show_idx = true;
            }
        } else {
            eprintln!("dirs: invalid argument");
            return 1
        }
    }
    if options.clear {
        shell.dirstack.clear();
    }
    if cmd.args.len() == 3 {
        let idx = if cmd.args[2].starts_with("+") {
            match cmd.args[2][1..].parse::<usize>() {
                Ok(int) => {
                    if int + 1 > shell.dirstack.len() {
                        eprintln!("dirs: stack not large enough");
                        return 2
                    } else {
                        int
                    }
                }
                Err(_) => {
                    eprintln!("dirs: invalid argument {}", cmd.args[2]);
                    return 2
                }
            }
        } else if cmd.args[2].starts_with("-") {
            match cmd.args[2][1..].parse::<usize>() {
                Ok(int) => {
                    if int + 1 > shell.dirstack.len() {
                        eprintln!("dirs: stack not large enough");
                        return 2
                    } else {
                        shell.dirstack.len() - 1 - int
                    }
                }
                Err(_) => {
                    eprintln!("dirs: invalid argument {}", cmd.args[2]);
                    return 2
                }
            }
        } else {
            eprintln!("dirs: invalid argument {}", cmd.args[2]);
            return 2
        };
        let path = match render_path(&shell.dirstack[idx], options.fullpath) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("dirs: error generating path");
                return 3;
            }
        };
        println!("{}", path);
    } else if cmd.args.len() == 2 {
        let mut display = String::new();
        for (i, path) in shell.dirstack.iter().enumerate() {
            let to_show = if options.show_idx {
                format!("[{}] {}", i, match render_path(path, options.fullpath) {
                    Ok(s) => s,
                    Err(_) => {
                        eprintln!("dirs: error generating path");
                        return 3
                    }
                })
            } else {
                match render_path(path, options.fullpath) {
                    Ok(s) => s,
                    Err(_) => {
                        eprintln!("dirs: error generating path");
                        return 3
                    }
                }
            };
            if options.per_line {
                println!("{}", to_show);
            } else {
                display.push_str(&to_show);
                display.push(' ');
            }
        }
        if !options.per_line {
            println!("{}", display);
        }
    } else {
        println!("{}", shell.dirstack.iter()
            .map(|path| render_path(path, options.fullpath)
                .unwrap_or(String::from(""))
            )
            .collect::<Vec<String>>().join(" ")
        )
    }
    0
}

pub fn render_path(path: &PathBuf, full: bool) -> Result<String, Box<dyn Error>> {
    let homedir = PathBuf::from(env::var("HOME")?);
    //TODO - FIXME: There is an unwrap here
    let to_display: String;
    if path.starts_with(&homedir) && !full {
        to_display = path.to_str()
            .unwrap()
            .replace(homedir.to_str().unwrap(), "~");
    } else {
        to_display = path.to_str().unwrap().to_string();
    }
    return Ok(to_display)
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
        if cmd.args[1] == "-n" {
            if shell.dirstack.len() < 2 {
                eprintln!("pushd: not enough elements in stack");
                return 2
            }
            let rem_idx = shell.dirstack.len() - 2;
            let path = shell.dirstack.remove(rem_idx);
            shell.dirstack.push(path);
            return 0
        }
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
        if shell.dirstack.len() < 2 {
            eprintln!("pushd: not enough elements in stack");
            return 2
        }
        let rem_idx = shell.dirstack.len() - 2;
        let path = shell.dirstack.remove(rem_idx);
        match shell.change_dir(&path) {
            Ok(_) => {},
            Err(_) => {
                eprintln!("dirs: cd error");
                return 3
            }
        }
        shell.dirstack.push(path);
        0
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