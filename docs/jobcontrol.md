## Job Control
Oyster also provides a job control mechanism, allowing the user to suspend jobs, or continue them in the background or foreground.

The main interface of this mechanism consists of the commands `jobs`, `fg` and `bg`.

While running a long running job, you can press Ctrl-Z to suspend the job. This sends a SIGTSTP signal to the job, halting it.
```
$ cogsy update
Beginning full database update.
Updating profile...     Success!
Updating wantlist...^Z (pressed Ctrl-Z here)
[1] 161489 cogsy Stopped
```
Once the job is halted, you can use either the `fg` or `bg` commands to continue. `fg` continues the command in the foreground, while `bg` sends it to the background.
```
$ fg
[1] cogsy
    Success!
Updating collection...  Success!
Writing to database...

Database update successful.
```
The number enclosed in the square brackets is the job's id.
`fg` and `bg` accept it as an argument, and require it if there is more than one suspended job.

To run a job in the background from the get-go, append `&` to the end of the command.

`cogsy update &`

The `jobs` command lists all the currenly suspended jobs.