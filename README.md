# Oyster

A simple (for now) UNIX shell written in Rust.

Right now it implements pipelining and conditional execution, as well as command aliasing and command substitution. It can also expand `~` to the user's home directory and expand variables, as well as expanding braces recursively. Its prompt can also display the last exit status, username and current working directory.

Right now, the most important things to implement are scripting, followed by tab completion and accessing past commands using the up key, as well as `!!` expansion and reading from an RC file (to set aliases and the prompt).

SLOC Count: `3004`

The final capabilities of this shell are:
- Pipelining, conditional execution, command substitution (Done)
- Command aliasing, variable, brace and tilde expansion (Done)
- Command substitution (done) and process substitution
- Stdin redirection and here documents/strings
- Job control - sending jobs to background (Done)
- Basic scripting (relies on bash/zsh to execute shell scripts)
    - I'm learning about programming languages now, and I might choose to develop this into an entire custom scripting language sometime down the line.
- Customizable prompt (oh-my-zsh/starship style) (half-done)
    - Can display last exit status, pwd, git status, time, etcetc.
- Nice-to-have builtins (history, env activation, etc)
- Completions (including some command completion)
See the notes file for more details.

I hope to work on this until it's something that can actually replace Zsh as my daily driver shell.