use std::env;
use std::error::Error;
use std::path::PathBuf;

pub const RESET: &str = "\x01\x1B[0m\x02";
// pub const BOLD: &str = "\x01\x1B[1m\x02";
// pub const UNDERLINED: &str = "\x01\x1B[4m\x02";

pub const BLUE: &str = "\x01\x1B[34m\x02";
// pub const BLACK: &str = "\x01\x1B[30m\x02";
// pub const WHITE: &str = "\x01\x1B[97m\x02";
pub const RED: &str = "\x01\x1B[31m\x02";
pub const GREEN: &str = "\x01\x1B[32m\x02";

// pub const BLUE_B: &str = "\x01\x1B[34m\x1B[1m\x02";
// pub const BLACK_B: &str = "\x01\x1B[30m\x1B[1m\x02";
pub const WHITE_B: &str = "\x01\x1B[97m\x1B[1m\x02";
// pub const RED_B: &str = "\x01\x1B[31m\x1B[1m\x02";
// pub const GREEN_B: &str = "\x01\x1B[32m\x1B[1m\x02";

// pub const BLUE_BG: &str = "\x01\x1B[44m\x02";
// pub const BLACK_BG: &str = "\x01\x1B[40m\x02";
// pub const WHITE_BG: &str = "\x01\x1B[107m\x02";
// pub const RED_BG: &str = "\x01\x1B[41m\x02";
// pub const GREEN_BG: &str = "\x01\x1B[42m\x02";

pub fn get_prompt(status: i32) -> String {
    let arrow: String;
    if status == 0 {
        arrow = format!("{}❯{}", GREEN, RESET);
    } else {
        arrow = format!("{}❯{}", RED, RESET);
    }
    if let Ok(prompt) = render_prompt() {
        return format!("{} \n{}", prompt, arrow);
    } else {
        eprintln!("oyster: could not generate prompt: directory error");
        return String::from(">>>>>")
    }
}

pub fn render_prompt() -> Result<String, Box<dyn Error>> {
    let current_dir = env::current_dir()?;
    let homedir = PathBuf::from(env::var("HOME")?);
    let user = env::var("USER")?;
    let to_display: String;
    if current_dir.starts_with(&homedir) {
        to_display = current_dir.to_str()
            .unwrap()
            .replace(homedir.to_str().unwrap(), "~");
    } else {
        to_display = current_dir.to_str().unwrap().to_string();
    }
    return Ok(format!("{}{}{}: {}{}{}",
            WHITE_B, 
            user, 
            RESET, 
            BLUE,
            to_display,
            RESET,
        ));
}