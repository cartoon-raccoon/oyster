# Oyster

A (relatively) simple UNIX shell written in Rust.

This was borne out of a desire to understand process execution in Linux, and slowly evolved into what it is today.

Trying to keep dependency count low, the only convenience crates used (as of now) are Regex, linefeed and glob.

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
    - Arrays that can be iterated over
- Script file interpretation (including rcfile reading)
- Basic builtins like `which`, `cd` and `alias`
- Directory stack manipulation
- Customizable prompt with last exit indication, username and PWD

## Planned
- Here documents/strings
- Associative arrays and namespaces
- Completions and history (including some command completion)
- Bangbang (`!!`) expansion to last command
- Fleshing out scripting system with more constructs
    - I'm learning about programming languages now, and I might choose to develop this into an entire custom scripting language sometime down the line.
- Additional logical AND and OR for if statements
- Switch statements
- Additional prompt customizability (git status, active environments, etc)
- Additional builtins (history, env activation, etc)
- Process substitution (tentative)

See the [documentation](docs/introduction.md) for more details.

This shell has been self-hosting since 19/10/2020, commit `f322fc3`. Every commit since then has been made with this shell.

I hope to work on this until it's something that can actually replace Zsh as my daily driver shell.