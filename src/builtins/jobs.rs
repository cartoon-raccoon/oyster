use crate::shell::Shell;
use crate::types::Cmd;

pub fn run(shell: &mut Shell, cmd: Cmd) -> i32 {
    if cmd.args.len() > 2 {
        eprintln!("jobs: too many arguments");
        return 2
    }
    if shell.jobs.is_empty() {
        println!("No jobs to print");
        return 0
    } else if cmd.args.len() == 2 {
        match cmd.args[1].parse::<i32>() {
            Ok(id) => {
                if let Some(job) = shell.get_job_by_id(id) {
                    println!("[{}] {} {} {}\n",
                    job.id, job.pgid, job.firstcmd, job.status);
                    return 0
                } else {
                    eprintln!("jobs: job with id {} not found", id);
                    return 1
                }
            }
            Err(_) => {
                eprintln!("jobs: please enter a numeric id");
                return 3
            }
        }
    }
    for (_id, job) in &shell.jobs {
        println!("[{}] {} {} {}\n",
        job.id, job.pgid, job.firstcmd, job.status);
    }
    0
}