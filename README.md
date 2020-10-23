# Oyster

A simple (for now) UNIX shell written in Rust.

Right now it implements pipelining and conditional execution, as well as command aliasing and command substitution. It can also expand ~ to the user's home directory and expand variables, as well as expanding braces recursively.

SLOC Count: `2112`

The final capabilities of this shell are:
- Pipelining, conditional execution, command substitution (Done)
- Command aliasing, variable, brace and tilde expansion (Done)
- Very basic scripting (relies on bash/zsh to execute shell scripts)
- Customizable prompt (oh-my-zsh/starship style)
    - Can display last exit status, pwd, git status, time, etcetc.
- Nice-to-have builtins (history, env activation, etc)
- Completions (including some command completion)
See the notes file for more details.

I hope to work on this until it's something that can actually replace Zsh as my daily driver shell.