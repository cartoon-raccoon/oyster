// pub enum Token {
//     Command,
//     Argument,
//     Builtin,
// }

#[derive(Debug, Clone)]
pub struct ParsedCmd<'a> {
    pub cmd: &'a str,
    pub args: Vec<&'a str>
}

pub struct Lexer {

}

impl Lexer {
    pub fn parse<'a>(line: &'a str) -> Option<ParsedCmd<'a>> {
        /* 
        * 1. Check if the first item is a builtin
        */
        let mut splitcmd = line.split_whitespace();
        if let Some(cmd) = splitcmd.nth(0) {
            Some(ParsedCmd {
                cmd: cmd,
                args: splitcmd.skip(0).collect()
            })
        } else {
            None
        }
    }
}