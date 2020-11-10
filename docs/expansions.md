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
After variables are expanded, they are automatically treated as a string by the shell. To operate on variables as their native type, the operation needs to be enclosed in a square bracket. See below.

See [Functions and Variables](functions.md) for more information on variables.

### Square Bracket Expansions

Square brackets have different meanings to the shell in different contexts.

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

In if statements, the square bracket is used to compare variables.
```
$ let int number = 69
$ if [$number < 420]
if > echo "it's not time yet"
if > end

it's not time yet
```
Outside of scripting, square brackets are used to operate on variables.

To operate on variables, enclose the operation inside a square bracket. This allows the shell to detect it and perform the operation.

The syntax for square bracket operations is as follows:

`[<operand> <operator> <operand>]`

The operands and operator must be separated by spaces, if not the shell cannot properly parse the contents.

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

Currently, there are 4 operations that can be performed on variables: Add, Subtract, Multiply and Divide. For strings, only Add can be performed, which concatenates the strings together. Any other operator will cause the shell to return an error.
```
$ echo ["hello" - "llo"]
oyster: operators other than `+` are not supported for strings
```

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

Right now, `*` cannot be backslash-escaped. To remove its meaning you need to enclose it in quotes. This is a bug and will be fixed.

### Brace Expansion
The shell can also expand braces. It accepts any brace with a list of words delimited by commas `,`. The list cannot be separated by spaces. A brace consists of a mandatory expansion and an optional prefix or suffix. For example:
```
$ echo button{.css,.js}
button.css button.js

$ echo un{trace,deni}able
untraceable undeniable
```
Braces can be nested and are expanded recursively.

Braces can be used in shell constructs too:
```
$ for i in un{trace,deni}able
for > echo $i
for > done
untraceable
undeniable
```