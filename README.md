# Oyster

A simple (for now) UNIX shell written in Rust.

Right now it implements pipelining and conditional execution, as well as command aliasing. It can also expand ~ to the user's home directory and expand variables as well.

SLOC Count: `1577`

The final capabilities of this shell are:
- Pipelining, conditional execution, command substitution
- Command aliasing, variable, brace and tilde expansion (with escaping)
- Very basic scripting (relies on bash/zsh to execute shell scripts)
- Customizable prompt (oh-my-zsh/starship style)
    - Can display last exit status, pwd, git status, time, etcetc.
- Nice-to-have builtins (history, env activation, etc)
- Completions (including some command completion)
See the notes file for more details.

I hope to work on this until it's something that can actually replace Zsh as my daily driver shell.