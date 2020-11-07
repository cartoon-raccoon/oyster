# Oyster

A (relatively) simple UNIX shell written in Rust.

This was borne out of a desire to understand process execution in Linux, and slowly evolved into what it is today.

Trying to keep dependency count low, the only convenience crates used (as of now) are Regex and glob.

SLOC Count: `3604`

Right now it implements:
- Pipelining and conditional execution
- IO Redirection: stdout to file, stdin from file, etc.
- Command aliasing and substitution
- Job control; sending jobs to and from background
- Tilde, variable and brace expansion
- Filepath globbing detection and expansion
- Basic scripting with for loops and if/elif/else statements
    - For loops can do range and glob expansion
- Defining and calling functions
- Customizable prompt with last exit indication, username and PWD

To implement:
- Process substitution
- Here documents/strings
- Completions and history (including some command completion)
- Script file interpretation (including rcfile reading)
- Bangbang (`!!`) expansion to last command
- Adding variables to scripting system
    - I'm learning about programming languages now, and I might choose to develop this into an entire custom scripting language sometime down the line.
- Additional if statement evaulation options, case statement
- Additional prompt customizability (git status, active environments, etc)
- Additional builtins (history, env activation, etc)

See the notes file for more details.

I hope to work on this until it's something that can actually replace Zsh as my daily driver shell.