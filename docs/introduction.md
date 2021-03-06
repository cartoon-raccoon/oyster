## Introduction
Welcome to Oyster, the UNIX shell that tries to keep it simple.

Oyster was named after the oyster, which has a shell, and also because of the saying "The world is your oyster." Like the saying, Oyster attempts to expose an interface that is pleasant and simple to use, while also empowering people to be ever more productive with while using the terminal.

Oyster is written in Rust, a systems programming language that provides memory safety, and compiles to native machine code.

Oyster is mostly a small side project that I work on in my spare time, and has a fraction of the features boasted by other more complex shells like bash, zsh or fish. However, the roadmap for this shell is clear (see notes) and I plan to implement all of them and more as time passes.

Oyster is not POSIX compliant. It has relatively few features compared to the POSIX shells like bash or zsh, and tries to simplify features that in POSIX shells some might find confusing to understand or use. It aims to be easy and simple to use with a focus on interactivity, and to this end heavily draws inspiration from Ion (the Redox OS shell) and some ideas from the fish shell. However, the base syntax (pipes and redirections, etc) are identical to the POSIX shells.

### On This Page
- Definitions
- Startup Sequence
- POSIX Compliance
- OS Compatibility

### Contents
1. [Command Execution](commands.md)
2. [Aliases and Expansions](expansions.md)
3. [Functions and Variables](functions.md)
4. [Scripting Constructs](scripting.md)
5. [Job Control](jobcontrol.md) 
6. [Builtin Commands](builtins.md)
7. [Prompt Customization](prompt.md)

## Definitions
Oyster has some concepts covered in its documentation that may need defining.

**Metacharacter**

A single character that has special meaning to the shell.
Enclosing a metacharacter in quotes may remove its meaning, depending on the quote type used.

E.g. double quotes (") remove the meaning of all metacharacters except backslashes, backticks and dollar signs, while single quotes (') remove the meaning of every metacharacter.

**Word**

Any sequence of characters delimited by a metacharacter.

Oyster parses groups of contiguous words into commands, and interprets the delimiting metacharacters to group the commands into pipelines and jobs.

**Command**

A single command that can be executed by a shell, parsed from a group of words.

e.g. `sudo pacman -Syu`

**Pipeline**

A group of commands separated by pipes `|`.

Oyster always builds commands into a pipeline. A single command is parsed into a pipeline containing one command.

e.g. `cat hello.txt | grep hello`

**Job**

A group of pipelines delimited by conditional execution markers `||`, `&&` or `;`.

e.g. `cat hello.txt | grep hello && sudo pacman -Syu`

**Scripting Construct**

A statement parsed from a set of jobs containing special keywords and following a structure that has special meaning to the shell.

The shell detects a construct immediately after parsing pipelines into jobs, and extracts the set of jobs that forms a complete scripting construct. It interprets and parses this set of jobs according to a separate set of rules, allowing the user to construct shell scripts.

e.g. This shell construct consists of an if statement nested inside a for loop.
```
for i in [1..5]; if echo "inside the loop"; echo "inside if"; end; done
```

**Function**

A user-defined set of jobs contained within the shell that is executed when called by the user.

Functions cannot be defined within a shell construct.

e.g. `execute()`

## Startup Sequence and RC File
When Oyster is invoked, it does a number of things. Firstly, it checks whether it is a login shell. Next, it logs all the environment variables with which it is launched. Finally, it accesses and reads its RC file.

The RC file is normally `~/.config/oyster/oshrc`, and is basically a shell script interpreted by the shell. In it contains various alias settings, env var exports, prompt definition, etc.

## POSIX Compliance
Oyster is already not POSIX-compliant, because of some of its syntax (the same contruct is used to expand ranges, operate on variables and test equality), and its limited feature set. Bash and zsh scripts definitely will not work on this shell.

Oyster is mainly designed for everyday interactive work such as running individual commands, and can provide some simple scripting when needed. When heavy lifting is needed in the form of a long, complex shell script, one can invoke Bash or zsh to run it, especially with the UNIX shebang.

~~I may or may not be using the lightweightedness of Oyster as an excuse for its limited features.~~

## OS Compatibility
Oyster will definitely not run on Windows. It makes raw system calls that are unique to or called differently on UNIX systems such as fork or execve (the two syscalls at the core of Oyster's functionality). In addition, the way it treats paths is different than Windows. The Rust standard library provides the `std::process::Command` type that is used to abstract away the kernel-level details, but Oyster does not use it.

Oyster should run perfectly well on any Linux system that has a Rust compiler and build system, as well as ncurses and sqlite3 installed (for the readline library and history expansion).

While macOS is technically a UNIX system, it may have some quirks that may require messing with the execution source code. Other parts of it should work properly.