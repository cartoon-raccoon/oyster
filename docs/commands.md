## Command Execution

### Input Processing
It is important to know how Oyster processes your input. It breaks down the input in three main passes:

1. Tokenization

Here the shell simply passes over the raw input string, detecting quotes and metacharacters, separating the string into tokens that encode information about the word type and the delimiters between them.

When the tokenizer detects a quote, cmdsub or bracket metacharacter, it enters a quoted state that exits only when the matching metacharacter is detected. If the tokenizer is still in the quoted state when it reaches the end of the line, it will continue to wait for input:
```
$ echo "hello
dquote > "
hello
```

2. Parsing

This is where the shell interprets the token stream outputted by the tokenizer, breaking the stream into jobs and commands, interpreting metacharacters, constructing the required data structures that encode such information. Glob expansion occurs here.

Similarly to quote detection in tokenization, the parser can detect scripting constructs and functions, and will continue to wait for more input until it detects the scripting construct is complete. It indicates this status with the `<construct> >` prompt:
```
$ for i in [1..5]
for > echo hello
for > done
hello
hello
hello
hello
```
This also applies to defining functions.

3. Expansion

This occurs right before execution. Here the shell detects the quote type of each word in each command, and performs expansions accordingly:

- If the word is unquoted, variable expansion, tilde expansion, and globbing expansion is performed.
- If the word is double quoted, only variable expansion is performed.
- If the word is single quoted, it is left unchanged.
- If the word is backquoted, command substitution occurs at this point, and the backquote is replaced by an unquoted single word. The resulting word is not tokenized or parsed, and is passed to the executor as is.

### Basic Execution
You can execute commands as you do on any other shell:
```
echo hello
sudo systemctl enable sddm.service
sudo pacman -Syu
```
### Pipelining
Pipelining is supported:
```
$ cat ~/Documents/stallman | grep interject
I'd just like to interject for a moment. What you're refering to as Linux, is in fact, GNU/Linux,

$ echo wassup my dudes | cowsay
 _________________ 
< wassup my dudes >
 ----------------- 
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||

```
The `|&` operator causes both stdout and stderr to be piped to the next command in the pipeline. It is equivalent to `2>&1`.
```
$ cat ~/Documents |& grep directory
cat: /home/sammy/Documents: Is a directory
```
### Conditional Execution
Oyster can execute a series of pipelines like any other shell:

`&&` indicates that the following command is executed only if the previous one succeeds.

`||` indicates that the following command is executed only if the previous command fails.

`;` indicates unconditional execution.

```
$ echo hello | grep hello && echo "this works!"
hello
this works!

$ cat ~/Documents || echo "this should get executed"
cat: /home/sammy/Documents: Is a directory
this should get executed
```
### IO Redirections
I/O redirection is supported similarly to Bash:

`>` and `>>` work as on any shell, but appending (`>>`) to a standard stream is not yet supported.

`$ cat ~/Documents 2>&1` (redirects stderr to stdout)

`$ echo hello &> hello` (redirects stdout and stderr to a file called hello)

Stdin redirection from a file is supported:

```
$ grep interject < ~/Documents/stallman
I'd just like to interject for a moment. What you're refering to as Linux, is in fact, GNU/Linux,
```

Planned: Process substitution (`<(command)`)

### Command Substitution
Oyster supports backtick and POSIX-style command substitution:
```
$ echo "hello my name is $(echo sam) and the time now is `date`"
hello my name is sam and the time now is Sun 08 Nov 2020 02:10:46 PM +08
```
As well as another style of substitution: `@` substitution, inspired by the Ion shell.

Backquote substitution is mostly more appropriate for inline substitutions. With POSIX-style and `@`-style substitutions, no variables are expanded and the entire enclosed string is treated as is. Backtick substitutions perform variable expansion.

When the shell detects a command substitution, it executes the enclosed command separately, with stdout piped back into it for collection. If it is a `@` substitution, it is replaced with a list of strings, which is the stdout split by whitespace. This allows substitution to be used as an iterable in for loops, as well as being used to expand to arguments for commands, e.g. `pacman -Rs @(pacman -Qqtd)`. Backquote substitution also splits by whitespace. POSIX-style `$` subsitution does not. 

`@` style substitution allows users to be more explicit about whether they want whitespace splitting to occur, instead of having to enclose their substitution inside quotes. Oyster still does support it however:

If the substitution occurs inside double quotes, the output is not split and is replaced in the quote as is.

If the substitution contains invalid syntax, the parser returns an error and the substitution is aborted.
```
$ echo `date |` (syntax error: pipe without second command)
error: command ends on delimiter
oyster: parse error in command substitution
```
