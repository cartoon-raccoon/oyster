//use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::parser::Lexer;
use crate::types::{
    Job, 
    ShellError, 
    Exec, 
    TokenCmd,
    Quote,
    Variable,
    TokenizeResult,
    ParseResult,
};
use crate::shell::{
    Shell,
};
use crate::expansion::{
    substitute_commands,
    expand_variables,
    expand_tilde,
    expand_braces,
    expand_range,
};
use crate::execute::{
    execute_jobs,
    execute as exec,
};
//TODO: Implement scoping; rn all variables have global scope

pub fn execute_scriptfile(shell: &mut Shell, filename: &str) -> Result<i32, ShellError> {
    let lines = BufReader::new(File::open(filename)?).lines();
    let mut status: i32 = 0;
    let mut buffer = String::new();
    for line in lines {
        let line = line?;
        if line.is_empty() || line.starts_with("#") {
            continue
        }
        buffer.push_str(line.trim());
        buffer.push('\n');
        let tokens = match Lexer::tokenize(&buffer)? {
            TokenizeResult::Good(parsedtokens) => parsedtokens,
            _ => {continue;}
        };
        let jobs = match Lexer::parse_tokens(shell, tokens)? {
            ParseResult::Good(jobs) => jobs,
            _ => {continue;}
        };
        match execute_jobs(shell, jobs, false) {
            Ok(result) => {
                status = result.0;
                buffer.clear();
            }
            Err(e) => {
                return Err(e)
            }
        }
    }
    Ok(status)
}

/// # Construct
/// 
/// A recursive type that forms an abstract syntax tree.
/// 
/// The `For` , `While` and `If` variants are branches that store the enum
/// itself alongside execution information, and the `Code` variant 
/// is the leaf (base case) that stores the actual commands to execute.
/// This allows `Construct` to recursively build itself
/// and then recursively execute itself.
/// Each variant implements its own behaviour, but ultimately
/// the branch variants call `execute()` on their own code.
/// 
/// The leaf variant evaluates its own jobs and returns
/// the exit status of the last command.
#[derive(Debug, Clone, PartialEq)]
pub enum Construct {
    /// Represents a `for` loop.
    For {
        loop_var: String,
        iterable: Vec<Variable>,
        code: Vec<Box<Construct>>,
    },
    /// Represents a `while` loop.
    While {
        condition: Job,
        code: Vec<Box<Construct>>,
    },
    /// Represents an `if/elif/else` statement.
    If {
        conditions: Vec<(Job, bool)>,
        code: Vec<Vec<Box<Construct>>>
    },
    /// The base case for everything.
    ///
    /// Does not contain itself.
    Code(Vec<Job>),
}

impl Construct {
    /// Recursively builds the AST from a raw Vec of jobs.
    pub fn build(shell: &mut Shell, mut raw: Vec<Job>) -> Result<Self, ShellError> {
        if raw.len() < 1 {
            return Err(ShellError::from("error: empty script"))
        }
        let len = raw.len();
        //only need to split after matching, because build() receives
        //a single construct that may contain many constructs on the same scope
        //but deeper inside
        if raw[0].execnext == Some(Exec::Background) {
            return Err(
                ShellError::from("oyster: invalid delimiter")
            )
        }
        match raw[0].cmds[0].cmd.1.as_str() {
            "for" => {
                if let Some(last) = raw.iter().last() {
                    //last.cmds should not be empty
                    if last.cmds[0].cmd.1 != "done" {
                        return
                        Err(
                            ShellError::from("oyster: could not parse script")
                        )
                    }
                }
                //removing "done"
                raw.remove(len - 1);
                //removing the "for" command
                let mut details = raw.remove(0).cmds.remove(0);
                if details.args.len() < 4 {
                    return
                    Err(
                        ShellError::from("oyster: insufficient arguments in for loop")
                    )
                }
                if details.args[2].1 != "in" {
                    return
                    Err(
                        ShellError::from("oyster: invalid for loop syntax")
                    )
                }

                let mut iterable = Vec::new();
                for word in &mut details.args[3..] {
                    if word.0 == Quote::SqBrkt {
                        iterable.extend(expand_range(shell, &word.1)?);
                    } else if word.0 == Quote::CmdSub {
                        if word.1.starts_with("$") {
                            iterable.push(substitute_commands(shell, &word.1)?);
                        } else if word.1.starts_with("@") {
                            let strings: Vec<String> = 
                            substitute_commands(shell, &word.1)?
                            .split_whitespace().map(|s| 
                                s.to_string()
                            ).collect();
                            iterable.extend(strings);
                        }
                    } else if word.0 == Quote::BQuote {
                        iterable.push(substitute_commands(shell, &word.1)?);
                    } else if word.0 == Quote::DQuote {
                        let mut string = word.1.clone();
                        expand_variables(shell, &mut string);
                        let string = substitute_commands(shell, &string)?;
                        iterable.push(string);

                    } else if word.1.starts_with("$") && word.0 == Quote::NQuote {
                        if let Some(var) = shell.get_variable(&word.1[1..]) {
                            iterable.push(var.to_string())
                        } else {
                            return Err(ShellError::from("oyster: variable not found"))
                        }

                    } else if word.1.starts_with("@") && word.0 == Quote::NQuote {
                        if let Some(var) = shell.get_variable(&word.1[1..]) {
                            if let Variable::Arr(arr) = var {
                                iterable.extend(arr.into_iter()
                                    .map(|var| var.to_string())
                                    .collect::<Vec<String>>()
                                )
                            } else {
                                return Err(ShellError::from("oyster: variable is not an array"))
                            }
                        } else {
                            return Err(ShellError::from("oyster: variable not found"))
                        }

                    } else if word.0 == Quote::CBrace {
                        let expanded = expand_braces(shell, word.1.clone())?
                            .into_iter().map(|mut s| {
                            expand_tilde(shell, &mut s);
                            s
                        }).collect::<Vec<String>>();
                        iterable.extend(expanded);
                    } else if word.0 == Quote::NmSpce {
                        //TODO
                        iterable.push(word.1.clone())
                    } else {
                        expand_tilde(shell, &mut word.1);
                        iterable.push(word.1.clone())
                    }
                }
                //TODO: match on Quote type
                let coden = split_on_same_scope(raw);
                let mut final_code = Vec::new();
                for construct in coden {
                    final_code.push(Box::new(Construct::build(shell, construct)?));
                }
                Ok(Construct::For {
                    loop_var: details.args[1].1.to_string(),
                    iterable: iterable
                        .into_iter()
                        .map(|s| Variable::from(s))
                        .collect(),
                    code: final_code,
                })
            }
            "while" => {
                if let Some(last) = raw.iter().last() {
                    //last.cmds should not be empty
                    if last.cmds[0].cmd.1 != "done" {
                        return
                        Err(
                            ShellError::from("oyster: could not parse script")
                        )
                    }
                }
                raw.remove(len - 1);
                let mut details = raw.remove(0);
                if details.cmds[0].args.len() < 2 {
                    return
                    Err(
                        ShellError::from("oyster: could not parse while loop")
                    )
                }
                let condition_cmd = details.cmds[0].args[1].clone();
                let condition_args = details.cmds[0].args[1..].to_vec();
                details.cmds[0].cmd = condition_cmd;
                details.cmds[0].args = condition_args;
                let coden = split_on_same_scope(raw);
                let mut final_code = Vec::new();
                for construct in coden {
                    final_code.push(Box::new(Construct::build(shell, construct)?));
                }
                Ok(Construct::While{
                    condition: details,
                    code: final_code,
                })
            }
            "if" => {
                let mut final_code = Vec::new();
                let mut conditions = Vec::new();
                for (condition, jobs) in split_on_branches(raw)? {
                    let mut code = Vec::new();
                    if let Some(condition) = condition {
                        conditions.push(condition);
                    }
                    for job in split_on_same_scope(jobs) {
                        code.push(Box::new(Construct::build(shell, job)?));
                    }
                    final_code.push(code);
                }
                Ok(Construct::If {
                    conditions: conditions,
                    code: final_code,
                })
            }
            n@ "done" | n@ "end" | n@ "elif" | n@ "else" => {
                return Err(
                    ShellError::from(format!("oyster: parse error near {}", n))
                )
            }
            _ => {
                return Ok(Construct::Code(raw))
            }
        }
    }
    /// Recursively executes itself, consuming itself as it goes.
    pub fn execute(
        self,
        shell: &mut Shell,
    ) -> Result<i32, ShellError> {

        match self {
            Construct::For {loop_var, iterable, code} => {
                let mut status: i32 = 0;

                for item in iterable {
                    shell.add_variable(&loop_var, Variable::from(item));
                    //* clone here slows things down a lot
                    let code2 = code.clone();
                    for block in code2 {
                        status = block.execute(shell)?;
                    }
                }
                shell.remove_variable(&loop_var);
                Ok(status)
            }
            Construct::While{condition, code} => {
                let mut status: i32 = 0;

                while eval_condition(shell, condition.clone())? {
                    let code2 = code.clone();
                    for block in code2 {
                        status = block.execute(shell)?;
                    }
                }

                Ok(status)
            }
            Construct::If {conditions, mut code} => {
                let mut status: i32 = 0;
                if code.is_empty() {
                    return Ok(0)
                }
                //note: this isn't particularly efficient
                for (condition, _execif) in conditions {
                    //for every if and elif
                    let to_exec = code.remove(0);
                    //if execif is true, execute the code block
                    if eval_condition(shell, condition)? {
                        for job in to_exec {
                            status = job.execute(shell)?;
                        }
                        return Ok(status)
                    }
                }
                //we have reached the else statement
                //at this point there should only be one item left in code
                //if code is empty, there is no else clause
                if code.len() == 1 {
                    let to_exec = code.remove(0);
                    for job in to_exec {
                        status = job.execute(shell)?;
                    }
                } else if code.len() > 1 {
                    return Err(
                        ShellError::from("error: code and condition mismatch")
                    )
                }
                Ok(status) //code is empty
            }
            //Base case
            Construct::Code(code) => {
                // for (key, value) in vars {
                //     shell.add_variable(&key, &value);
                // }
                let (status, _cap) = execute_jobs(shell, code, false)?;
                Ok(status)
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum EqTest {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

fn eval_condition(shell: &mut Shell, mut condition: Job) 
-> Result<bool, ShellError> {
    use EqTest::*;
    if condition.cmds.len() == 1 &&
       condition.cmds[0].cmd.0 == Quote::SqBrkt {
        if condition.cmds[0].cmd.1.trim() == "true" {
            return Ok(true)
        } else if condition.cmds[0].cmd.1.trim() == "false" {
            return Ok(false)
        }
        let condition = condition.cmds.remove(0).cmd.1;
        let (lhs, eq, rhs) = tokenize_sqbrkt(shell, condition)?;
        if Variable::types_match(&lhs, &rhs) {
            match eq {
                Eq => {return Ok(lhs == rhs)}
                Ne => {return Ok(lhs != rhs)}
                Lt => {return Ok(lhs  < rhs)}
                Gt => {return Ok(lhs  > rhs)}
                Le => {return Ok(lhs <= rhs)}
                Ge => {return Ok(lhs >= rhs)}
            }
        } else {
            return Err(
                ShellError::from(
                    "oyster: cannot compare variables of different types"
                )
            )
        }
    } else if condition.cmds[0].cmd.0 == Quote::CmdSub {
        let lhs = Variable::Str(substitute_commands(
            shell, 
            &condition.cmds[0].args[0].1
        )?);
        let op = if let Some(eqtest) 
        = get_valid_operator(&condition.cmds[0].args[1].1) {
            eqtest
        } else {
            return Err(ShellError::from("oyster: invalid operator"))
        };
        let rhs = Variable::Str(condition.cmds[0].args[2].1.clone());
        match op {
            Eq => {Ok(lhs == rhs)}
            Ne => {Ok(lhs != rhs)}
            Lt => {Ok(lhs  < rhs)}
            Gt => {Ok(lhs  > rhs)}
            Le => {Ok(lhs <= rhs)}
            Ge => {Ok(lhs >= rhs)}
        }
    } 
    else {
        return Ok(exec(shell, condition, false, false)?.status == 0)
    }
}

fn tokenize_sqbrkt(shell: &mut Shell, condition: String) 
-> Result<(Variable, EqTest, Variable), ShellError> {
    let mut in_quote = false;
    let mut parsed: Vec<(bool, String)> = Vec::new();
    let mut word = String::new();
    for c in condition.trim().chars() {
        match c {
            '"' => {
                if !in_quote {
                    in_quote = true;
                } else {
                    parsed.push((true, word.clone()));
                    word.clear();
                    in_quote = false;
                }
            }
            ' ' => if !in_quote {
                if !word.is_empty() {
                    parsed.push((false, word.clone()));
                }
                word.clear();
            }
            _ => {
                word.push(c);
            }
        }
    }
    if !word.is_empty() {
        parsed.push((false, word));
    }
    if parsed.len() != 3 {
        return Err(
            ShellError::from(
                "oyster: cannot parse square bracket"
            )
        )
    }
    if parsed[1].0 {
        return Err(
            ShellError::from(
                "oyster: middle operator is quoted"
            )
        )
    }
    let lhs = if parsed[0].0 {
        Variable::Str(parsed[0].1.clone())
    } else if parsed[0].1.starts_with("$") {
        if let Some(var) = shell.get_variable(&parsed[0].1[1..]) {
            var
        } else {
            return Err(
                ShellError::from(
                    format!("oyster: variable {} not found", &parsed[0].1[1..])
                )
            )
        }
    } else {
        Variable::from(&parsed[0].1)
    };
    let rhs = if parsed[2].0 {
        Variable::Str(parsed[2].1.clone())
    } else if parsed[2].1.starts_with("$") {
        if let Some(var) = shell.get_variable(&parsed[2].1[1..]) {
            var
        } else {
            return Err(
                ShellError::from(
                    format!("oyster: variable {} not found", &parsed[2].1[1..])
                )
            )
        }
    } else {
        Variable::from(&parsed[2].1)
    };
    let comparator = if let Some(eqtest) = get_valid_operator(&parsed[1].1) {
        eqtest
    } else {
        return Err(
            ShellError::from("oyster: invalid operator")
        )
    };
    Ok((lhs, comparator, rhs))
}

fn get_valid_operator(op: &str) -> Option<EqTest> {
    match op {
        "==" | "-eq" => { Some(EqTest::Eq) }
        "!=" | "-ne" => { Some(EqTest::Ne) }
        "<"  | "-lt" => { Some(EqTest::Lt) }
        ">"  | "-gt" => { Some(EqTest::Gt) }
        "<=" | "-le" => { Some(EqTest::Le) }
        ">=" | "-ge" => { Some(EqTest::Ge) }
        _ => { None }
    }
}

/// Splits a single scope into its individual constructs
fn split_on_same_scope(raw: Vec<Job>) -> Vec<Vec<Job>> {
    let mut to_return = Vec::new();
    let mut buffer = Vec::new();
    let mut nesting_level: usize = 0;
    for job in raw {
        match job.cmds[0].cmd.1.as_str() {
            "for" | "if" | "while" => {
                buffer.push(job);
                nesting_level += 1;
            }
            "done" | "end" => {
                buffer.push(job);
                nesting_level -= 1;
                if nesting_level == 0 {
                    to_return.push(buffer.clone());
                    buffer.clear();
                }
            }
            _ => {
                buffer.push(job);
            }
        }
    }
    if buffer.len() > 0 {
        to_return.push(buffer);
    }
    to_return
}

/// Splits an if statement into if/elif/else blocks
//TODO: optimise this (i.e. remove clones `urgh`)
//TODO2: Fix ! condition
fn split_on_branches(raw: Vec<Job>) 
             //condition    codeblock  
-> Result<Vec<(Option<(Job, bool)>, Vec<Job>)>, ShellError> {
    let mut to_return = Vec::<(Option<(Job, bool)>, Vec<Job>)>::new();
    let mut buffer = Vec::new();
    let mut nesting_level: isize = -1;
    let mut condition = None;
    let mut raw = raw.into_iter();
    while let Some(job) = raw.next() {
        match job.cmds[0].cmd.1.as_str() {
            "for" | "while" => {
                buffer.push(job);
                nesting_level += 1;
            }
            "if" => { 
                nesting_level += 1;
                if job.cmds[0].args.len() < 2 {
                    return Err(ShellError::from("oyster: syntax error (no condition)"))
                }
                if nesting_level == 0 {
                    let command = if job.cmds[0].args[1].1.starts_with("!") {
                        (
                            job.cmds[0].args[1].0, 
                            job.cmds[0].args[1].1[1..].to_string()
                        )
                    } else {
                        job.cmds[0].args[1].clone()
                    };
                    condition = Some((Job {
                        cmds: vec![TokenCmd {
                            cmd: command,
                            args: job.cmds[0].args[1..].to_vec(),
                            redirects: job.cmds[0].redirects.clone(),
                            pipe_stderr: job.cmds[0].pipe_stderr,
                        }],
                        execnext: job.execnext,
                        id: job.id,
                        },
                        !job.cmds[0].args[1].1.starts_with("!"))
                    );
                } else {
                    buffer.push(job);
                }
            }
            "elif" => {
                if job.cmds[0].args.len() < 2 {
                    return Err(ShellError::from("oyster: syntax error (no condition)"))
                }
                if nesting_level == 0 {
                    if job == Job::default() {
                        return Err(
                            ShellError::from(
                                "oyster: syntax error near `elif`"
                            )
                        )
                    }
                    to_return.push(
                        (condition.clone(), 
                        buffer.clone()));
                    buffer.clear();
                    let command = if job.cmds[0].args[1].1.starts_with("!") {
                        (
                            job.cmds[0].args[1].0, 
                            job.cmds[0].args[1].1[1..].to_string()
                        )
                    } else {
                        job.cmds[0].args[1].clone()
                    };
                    condition = Some((Job {
                        cmds: vec![TokenCmd {
                            cmd: command,
                            args: job.cmds[0].args[1..].to_vec(),
                            redirects: job.cmds[0].redirects.clone(),
                            pipe_stderr: job.cmds[0].pipe_stderr,
                        }],
                        execnext: job.execnext,
                        id: job.id,
                        },
                        !job.cmds[0].args[1].1.starts_with("!"))
                    );
                } else {
                    buffer.push(job);
                }
            }
            "else" => {
                if nesting_level == 0 {
                    to_return.push(
                        (condition.clone(), 
                        buffer.clone()));
                    buffer.clear();
                    condition = None;
                } else {
                    buffer.push(job);
                }
            }
            "done" => {
                buffer.push(job);
                nesting_level -= 1;
            }
            "end" => {
                if nesting_level == 0 {
                    to_return.push((condition.clone(), buffer.clone()));
                    buffer.clear();
                }
                nesting_level -= 1;
            }
            _ => {
                buffer.push(job);
            }
        }
    }
    if nesting_level > 0 {
        return Err(
            ShellError::from("oyster: parse error in if statement")
        )
    }
    //println!("{:?}", to_return);
    
    Ok(to_return)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scriptfile_exec() {
        let mut shell = Shell::new();
        assert_eq!(
            execute_scriptfile(&mut shell, "testscripts/test1").unwrap(),
            0
        )
    }

    #[test]
    fn test_sqbrkt_tokenizer() {
        let teststring = String::from("$thing == \"hello\"");
        let teststring2 = String::from("$number -ne 2");
        let mut shell = Shell::new();
        shell.add_variable("thing", Variable::from("hello"));
        shell.add_variable("number", Variable::from("2"));
        assert_eq!(
            tokenize_sqbrkt(&mut shell, teststring).unwrap(),
            (Variable::from("hello"), EqTest::Eq, Variable::Str(String::from("hello")))
        );
        assert_eq!(
            tokenize_sqbrkt(&mut shell, teststring2).unwrap(),
            (Variable::from("2"), EqTest::Ne, Variable::Int(2))
        )
    }
}