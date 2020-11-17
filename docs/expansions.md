## Aliases and Expansions
### Command Aliasing
Oyster supports command aliasing using the builtin `alias` command. The syntax is similar to zsh:

`alias addpkg = "sudo pacman -S"`

Also equivalent: `alias addpkg "sudo pacman -S"`

(The equal sign is optional.)

There are spaces between the equal sign and the two arguments.
```
$ alias greet = "echo hello | cowsay -f tux"
$ greet
 _______ 
< hello >
 ------- 
   \
    \
        .--.
       |o_o |
       |:_/ |
      //   \ \
     (|     | )
    /'\_   _/`\
    \___)=(___/

```
See [builtins](builtins.md) for more information.

### Variable Expansions

The shell can store pieces of data called variables. These are also used in the shell's programming language.

Variables are expanded by prepending `$` to the word that is the name of the variable.

```
$ let hello = "howdy pardner"
$ echo $hello
howdy pardner
```
The shell can detect variables in the middle of words. It reads from the `$` all the way up to the next character that is not alphanumeric, and treats that as the name of the variable.

Arrays can also be expanded to a list of strings using the `@` notation. However, this cannot be detected in the middle of a word; a word has to start with it for it to be detected.

After variables are expanded, they are automatically treated as a string by the shell. To operate on variables as their native type, the operation needs to be enclosed in a square bracket. See below.

See [Functions and Variables](functions.md) for more information on variables.

### Square Bracket Expansions

Square brackets have different meanings to the shell in different contexts. In POSIX shells, square brackets are only used to evaluate conditions, but here they are used for range expansion, equality testing and variable operations.
```
$ for i in [1..=5]
for > echo [$i + 4]
for > if [$i == 3]
if > echo i is 3
if > end
for > done

5
6
7
i is 3
8
9
```
This example shows square brackets being used differently in a single construct.

When used in scripting constructs, their meanings again differ depending on the construct used. 

In for loops, the square bracket expands to a range.

`[1..=5]` is semantically equivalent to `1 2 3 4 5`.
```
$ for i in [1..=5]
for > echo counting to $i
for > done

counting to 1
counting to 2
counting to 3
counting to 4
counting to 5
```

In if statements and while loops, the square bracket is used to compare variables.
```
$ let int number = 69
$ if [$number < 420]
if > echo "it's not time yet"
if > end

it's not time yet
```
Outside of scripting, square brackets are used to operate on variables.

To operate on variables, enclose the operation inside a square bracket. This allows the shell to detect it and perform the operation, replacing the bracket's contents with the result of the operation.

The syntax for square bracket operations is as follows:

`[<operand> <operator> <operand>]`

The operands and operator must be separated by spaces, if not the shell cannot properly parse the contents. If the shell cannot parse the contents, the square bracket will not be expanded, and instead the contents of the bracket will be returned with the surrounding brackets.
```
$ echo [hello]
[hello]
```
Currently, there are 4 operations that can be performed on variables: Add, Subtract, Multiply and Divide. For strings, only Add can be performed, which concatenates the strings together. Any other operator will cause the shell to return an error.
```
$ echo ["hello" - "llo"]
oyster: operators other than `+` are not supported for strings
```
Arrays cannot be operated on in any way, the shell will throw an error if attempting to do so.

Operate-and-assign operators exist in the source code, but they only currently return a "not supported" error. They will likely be implemented as part of the `let` command.

The operand can be a literal, in which case the type is inferred, or can be a variable, designated with a `$`. If there is no `$`, the operand is treated as a literal.

_Tip:_ To force the shell to treat the literal as a string, you can surround it in quotes.
```
$ echo ["1" + "2"]
12 (concatenation instead of addition)
```
Other examples:
```
$ let pi = 3.141
$ let e = 2.718
$ echo [$pi * $e]
8.537238

$ echo [one + two]
onetwo
```
If the operand is a variable, it is expanded before being operated on.
Both operands are type checked before the operation is performed. If the types don't match, the shell returns an error.

### Tilde Expansions
Oyster can also do tilde expansions.

The standard tilde `~` expands to the user's home directory:
```
$ echo ~/Documents/stallman
/home/sammy/Documents/stallman
```
The tilde with a plus `~+` expands to the current working directory:
```
$ pwd
/home/sammy/Projects/oyster

$ echo ~+/src
/home/sammy/Projects/oyster/src
```
### Globbing Expansion
If a word contains the characters `/` or `*`, it is treated as a path to be glob-expanded and is replaced with a list of arguments corresponding to the glob expansion of the path.

`*` lists all the files in that directory that match the pattern.
`**` recursively lists all the files in the directory and its subdirectories.

Glob expansions can be used in for loops:
```
$ for i in ~/*
for > echo $i
for > done

/home/sammy/Desktop
/home/sammy/Documents
/home/sammy/Downloads
/home/sammy/Music
/home/sammy/Pictures
/home/sammy/Projects
/home/sammy/Videos
```

Right now, `*` cannot be backslash-escaped. To remove its meaning you need to enclose it in quotes. This is a bug and will be fixed.

### Brace Expansion
The shell can also expand braces. It accepts any brace with a list of words delimited by commas `,`. The list cannot be separated by spaces. A brace consists of a mandatory expansion and an optional prefix or suffix. For example:
```
$ echo button{.css,.js}
button.css button.js

$ echo un{trace,deni}able
untraceable undeniable
```
Braces can be nested and are expanded recursively:
```
$ echo 1{2,{3,4},5}6
126 136 146 156
```

Braces can also be expanded into ranges. If a brace has no prefix or suffix, and contains the substring `..`, it is automatically treated as a range to be expanded.

Ranges expand to an array of numbers in the same fashion as square bracket ranges:

`{1..10}` expands to `1 2 3 4 5 6 7 8 9`.

To include the last number, use `=`:

`{1..=10}` expands to `1 2 3 4 5 6 7 8 9 10`.

Ranges can also be descending and stepped over:

`{10..1}` expands to `10 9 8 7 6 5 4 3 2`.

`{0..10..2}` expands to `0 2 4 6 8`.

Braces expand to iterables, and thus can be used in for loops:
```
$ for i in un{trace,deni}able
for > echo $i
for > done
untraceable
undeniable
```