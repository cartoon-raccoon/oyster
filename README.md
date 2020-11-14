# Oyster

A (relatively) simple UNIX shell written in Rust.

This was borne out of a desire to understand process execution in Linux, and slowly evolved into what it is today.

Trying to keep dependency count low, the only convenience crates used (as of now) are Regex and glob.

SLOC Count: `5263`

## Features
- Pipelining and conditional execution
- IO Redirection: stdout to file, stdin from file, etc.
- Command aliasing and substitution
- Tilde, variable and brace expansion
- Filepath globbing detection and expansion
- Defining and calling functions
- Job control; sending jobs to and from background
- Basic scripting with for loops and if/elif/else statements
    - Variables are typed and can be operated on
- Script file interpretation (including rcfile reading)
- Basic builtins like `which`, `cd` and `alias`
- Directory stack
- Customizable prompt with last exit indication, username and PWD

## Planned
- Process substitution
- Here documents/strings
- Completions and history (including some command completion)
- Bangbang (`!!`) expansion to last command
- Fleshing out scripting system with more constructs
    - I'm learning about programming languages now, and I might choose to develop this into an entire custom scripting language sometime down the line.
- Additional if statement evaluation options, case statement
- Additional prompt customizability (git status, active environments, etc)
- Additional builtins (history, env activation, etc)

See the [documentation](docs/introduction.md) for more details.

I hope to work on this until it's something that can actually replace Zsh as my daily driver shell.