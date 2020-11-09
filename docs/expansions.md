## Aliasing, Functions, Variables, and Expansions
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
### Functions
Similar to aliases are shell functions. These are defined with the `func` keyword, and are called with the name and `()` concatenated to the back.
To denote the end of the function, you must use the `endfn` keyword.
As with scripting constructs, the shell will wait for further input if it detects the function is not yet complete.
```
$ func say_hello
func > echo Hi!
func > echo What\'s popping?
func > echo bye!
func > endfn

$ say_hello()
Hi!
What 's popping?
bye!
```
Functions cannot be defined inside scripting constructs:
```
$ for i in [1..5];
for > func thing
error: cannot define function in shell construct
```
Functions can also accept parameters. The number of parameters they accept is defined in the function definition, after the function name:
```
$ func say_hi 2
func > echo $say_hi0
func > echo $say_hi1
func > endfn
```
The parameters can be accessed with the variables `$<function><number>`, where function is the function name and number is its index in the parameter vector.
To call a function with its parameters, call the function and place its parameters after the function call:
```
$ say_hi() hello there
hello
there
```
If the number of parameters passed and the number of parameters specified do not match, the function will return an error. Functions defined without a parameter count are automatically variadic and can accept any number of functions. If there are more variables specified in the function definition than parameters passed, the missing variables will expand to empty strings.

All standard expansions such as variable expansion, command substitution and brace expansion, except globbing will work on function parameters.

Oyster also supports calling functions within functions:
```
$ func hello
func > echo hello
func > hello2()
func > endfn 

$ func hello2
func > echo "hello again"
func > endfn 

$ hello()
hello
hello again
```

As such, recursive functions can be defined in Oyster. To prevent stack overflows in the shell itself, Oyster has an internal stack that grows every time a function is called, and shrinks when a function ends. Oyster also has a maximum recursion depth that is checked every time a function is called. Once this depth is exceeded, Oyster will automatically return an error.

### Variables
The shell can also accept user-defined variables. Variables have types; there are three main types: Str (string), Int (integer) and Flt (float).

There are two ways to define variables: implicitly, and with the `let` command.
```
<name>=<value>
let <type> <name> = <value>
```
The second way is the only way to assign quoted text. Variable types are mostly inferred: the first way will automatically infer types, and `let` will infer types if `<type>` is not specified. 

Type inference works as follows: The shell attempts to parse the value as an int. If it fails, the shell then tries to parse as a float. If that too fails, the shell parses the value as a string.
```
let str number = 2.4 (can be parsed to flt, but assigned as str)
let int number2 = 5 (parsed as int)
let number3 = 3.14 (inferred as flt)
let text = "hello" (inferred as str)
```
If the type is specified but the value cannot be parsed as that type, `let` returns an error.

Expanding variables is similar to other shells: use `$`;
```
$ let hello = "howdy pardner"
$ echo $hello
howdy pardner
```

As of now, `$` cannot be backslash-escaped. The only way to use a literal $ is to enclose it in single quotes (variable expansion is performed on double quotes). This is a bug and will be fixed.

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