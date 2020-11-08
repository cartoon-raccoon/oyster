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
Func > echo Hi!
Func > echo What\'s popping?
Func > echo bye!
Func > endfn

$ say_hello()
Hi!
What 's popping?
bye!
```
Functions cannot be defined inside scripting constructs:
```
$ for i in [1..5];
For > func thing
error: cannot define function in shell construct
```
Functions can also accept parameters. The number of parameters they accept is defined in the function definition, after the function name:
```
$ func say_hi 2
Func > echo $f0
Func > echo $f1
Func > endfn
```
The parameters can be accessed with the variables `$f<number>`, where the number is its index in the parameter vector.
To call a function with its parameters, call the function and place its parameters after the function call:
```
$ say_hi() hello there
hello
there
```
If the number of parameters passed and the number of parameters specified do not match, the function will return an error. Functions defined without a parameter count are automatically variadic.

All standard expansions except globbing will work on function parameters.

### Variables
The shell can also accept user-defined variables. Right now, all user defined variables are treated as strings, but variable typing is planned.

There are two ways to define variables:
```
hello=wassup
let hello = "wassup dude"
```
The second way is the only way to assign quoted text and is also the only way to specify variable types (coming soon). The first way defaults to str type.

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
For > echo $i
For > done
untraceable
undeniable
```