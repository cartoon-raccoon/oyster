## Introduction
Welcome to Oyster, the UNIX shell that tries to keep it simple.

Oyster was named after the oyster, which has a shell, and also because of the saying "The world is your oyster."

Oyster is written in Rust, a systems programming language that provides memory safety, and compiles to native machine code.

Oyster is mostly a small side project that I work on in my spare time, and has a fraction of the features boasted by other more complex shells like bash, zsh or fish. However, the roadmap for this shell is clear (see notes) and I plan to implement all of them and more as time passes.

### On This Page
- Definitions
- Startup Sequence

### Contents
1. [Commands](commands.md)
2. [Expansions](expansions.md)
3. [Job Control](jobcontrol.md)
4. [Scripting](scripting.md)
5. [Prompt](prompt.md)

## Definitions
While this shell is mostly syntactically similar to Bash or Zsh, it does draw some ideas from the fish shell and Ion (the shell for Redox OS). It also uses some terms in its documentation that may need defining.

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

A group of commands delimited by pipes `|`.

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
for i in [1..5]; if echo "inside the loop; echo "inside if"; end; done
```

**Function**

A user-defined set of jobs contained within the shell that is executed when called by the user.

Functions cannot be defined within a shell construct.

e.g. `execute()`

## Startup Sequence and RC File
When Oyster is invoked, it does a number of things. Firstly, it checks whether it is a login shell. Next, it logs all the environment variables with which it is launched. Finally, it accesses and reads its RC file.

The RC file is normally `~/.config/oyster/oshrc`, and is basically a shell script interpreted by the shell. In it contains various alias settings, env var exports, prompt definition, etc.