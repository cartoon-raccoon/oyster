use std::env;
use std::error::Error;
use std::path::PathBuf;

use nix::unistd::gethostname;

pub const RESET: &str = "\x01\x1B[0m\x02";
pub const BOLD: &str = "\x01\x1B[1m\x02";
pub const UNDERLINED: &str = "\x01\x1B[4m\x02";

pub const BLUE: &str = "\x01\x1B[34m\x02";
pub const YELLOW: &str = "\x01\x1B[33m\x02";
pub const BLACK: &str = "\x01\x1B[30m\x02";
pub const WHITE: &str = "\x01\x1B[97m\x02";
pub const RED: &str = "\x01\x1B[31m\x02";
pub const GREEN: &str = "\x01\x1B[32m\x02";

pub const BLUE_B: &str = "\x01\x1B[34m\x1B[1m\x02";
pub const YELLOW_B: &str = "\x01\x1B[33m\x1B[1m\x02";
pub const BLACK_B: &str = "\x01\x1B[30m\x1B[1m\x02";
pub const WHITE_B: &str = "\x01\x1B[97m\x1B[1m\x02";
pub const RED_B: &str = "\x01\x1B[31m\x1B[1m\x02";
pub const GREEN_B: &str = "\x01\x1B[32m\x1B[1m\x02";

pub const OYSTER_DFL: &str 
    = "{BOLD}{USER}{RESET}: {CWD_TOP} {COLOR_ST}>>{RESET} ";


pub fn render_prompt(status: i32) -> String {
    let prompt = match env::var("OYSTER_PROMPT") {
        Ok(string) => string,
        Err(_) => OYSTER_DFL.to_string()
    };
    let mut text = Vec::new();
    let mut tokens = Vec::new();
    let mut word = String::new();

    let mut prompt = prompt.chars();
    
    while let Some(c) = prompt.next() {
        match c {
            '{' => {
                text.push(word.clone());
                word.clear();
            }
            '}' => {
                tokens.push(word.clone());
                word.clear();
            }
            '\\' => {
                if let Some(c) = prompt.next() {
                    word.push(c);
                    continue;
                }
            }
            _ => {
                word.push(c);
            }
        }
    }
    text.push(word);

    let mut final_prompt = String::new();
    let mut text_iter = text.iter();
    if let Some(text) = text_iter.next() {
        final_prompt.push_str(text);
    }
    for token in tokens {
        match token.as_str() {
            "CWD_FULL" => {
                match render_cwd(false) {
                    Ok(dir) => {final_prompt.push_str(&dir);}
                    Err(_) => {
                        eprintln!("oyster: error getting pwd");
                        return String::from(">>>>> ");
                    }
                }
            }
            "CWD_TOP" => {
                match render_cwd(true) {
                    Ok(dir) => {final_prompt.push_str(&dir);}
                    Err(_) => {
                        eprintln!("oyster: error getting pwd");
                        return String::from(">>>>> ");
                    }
                }
            }
            "USER" => {
                match env::var("USER") {
                    Ok(user) => {final_prompt.push_str(&user);}
                    Err(_) => {
                        eprintln!("oyster: error getting user");
                        final_prompt.push_str(">>>>> ");
                    }
                }
            }
            "HOST" => {
                let mut buf = [0u8; 64];
                match gethostname(&mut buf) {
                    Ok(cstr) => {
                        match cstr.to_str() {
                            Ok(hsnm) => {
                                final_prompt.push_str(hsnm);
                            }
                            Err(e) => {
                                eprintln!("oyster: cstr err {}", e)
                            }
                        }
                    }
                    Err(_) => {
                        eprintln!("oyster: error getting hostname");
                        final_prompt.push_str(">>>>> ");
                    }
                }
            }
            "GIT_REPO" => {
                //TODO: get git status
            }
            "NEWLINE" => {
                final_prompt.push('\n');
            }
            "COLOR_ST" => {
                if status == 0 {
                    final_prompt.push_str(GREEN);
                } else {
                    final_prompt.push_str(RED);
                }
            }
            "BLUE" => {
                final_prompt.push_str(BLUE);
            }
            "YELLOW" => {
                final_prompt.push_str(YELLOW);
            }
            "BLACK" => {
                final_prompt.push_str(BLACK);
            }
            "WHITE" => {
                final_prompt.push_str(WHITE);
            }
            "RED" => {
                final_prompt.push_str(RED);
            }
            "GREEN" => {
                final_prompt.push_str(GREEN);
            }
            "BLUE_B" => {
                final_prompt.push_str(BLUE_B);
            }
            "YELLOW_B" => {
                final_prompt.push_str(YELLOW_B);
            }
            "BLACK_B" => {
                final_prompt.push_str(BLACK_B);
            }
            "WHITE_B" => {
                final_prompt.push_str(WHITE_B);
            }
            "RED_B" => {
                final_prompt.push_str(RED_B);
            }
            "GREEN_B" => {
                final_prompt.push_str(GREEN_B);
            }
            "BOLD" => {
                final_prompt.push_str(BOLD);
            }
            "ULINED" => {
                final_prompt.push_str(UNDERLINED);
            }
            "RESET" => {
                final_prompt.push_str(RESET);
            }
            n @ _ => {
                eprintln!("oyster: error parsing prompt - unknown token {}", n);
            }
        }
        if let Some(text) = text_iter.next() {
            final_prompt.push_str(text);
        }
    }
    final_prompt
}

pub fn render_cwd(last: bool) -> Result<String, Box<dyn Error>> {
    let current_dir = env::current_dir()?;
    let homedir = PathBuf::from(env::var("HOME")?);
    //TODO - FIXME: There is an unwrap here
    let path_end = current_dir.iter().last()
        .unwrap().to_owned().into_string().unwrap();
    if last {
        if homedir == PathBuf::from(current_dir) {
            return Ok(String::from("~"))
        } else {
            return Ok(path_end)
        }
    }
    let to_display: String;
    if current_dir.starts_with(&homedir) {
        to_display = current_dir.to_str()
            .unwrap()
            .replace(homedir.to_str().unwrap(), "~");
    } else {
        to_display = current_dir.to_str().unwrap().to_string();
    }
    return Ok(to_display)
}