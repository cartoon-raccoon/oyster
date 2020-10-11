pub enum Token {
    Command,
    Argument,
}

pub struct ParsedCmd<'a> {
    cmd: &'a str,
    args: Vec<&'a str>
}

pub struct Lexer {

}

impl Lexer {
    pub fn parse<'a>(line: &'a str) -> Option<ParsedCmd<'a>> {
        let mut splitcmd = line.split_whitespace();
        if let Some(cmd) = splitcmd.nth(1) {
            Some(ParsedCmd {
                cmd: cmd,
                args: splitcmd.skip(1).collect()
            })
        } else {
            None
        }
    }
}