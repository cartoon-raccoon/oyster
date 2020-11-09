## Builtins
Oyster offers a few builtin commands, for convenience but also some of which are crucial to the shell's operation.

### Alias
This command allows you to set aliases for commands. This gets expanded at parse time, but they should still work in shell scripts and functions. The syntax is:

`alias <alias-name> = "<aliased-command>"`

The aliased command will most likely have to be in quotes, unless there are no spaces in it. The equals sign is also optional.

Examples:
```
$ alias kittyconf = "nano ~/.config/kitty/kitty.conf"

$ alias updatepkgs = "~/Documents/updatepkgs"

$ alias rusthelp "rustup docs --book"
```
Alias will take the command to be aliased and tokenize and parse it, stopping short of executing it. If tokenization and parsing fail, alias will return an error and the command alias will not be added. Note that this failure is a failure of the shell syntax itself, and errors such as missing arguments will still pass.
```
$ alias hello = "echo hello |"
oyster: bad assignment for `hello`
```
The `unalias` command removes an alias from the shell. If the specified alias doesn't exist, the shell returns an error.

### Export
This command exports variables to the environment, essentially adding environment variables that processed spawned by the shell will inherit.
```
export EDITOR = "/usr/bin/nano"
export VISUAL = "/usr/bin/nano"
export OYSTER_PROMPT = "{YELLOW_B}[{HOST}] {USER}{RESET}: {BLUE}{CWD_FULL}{RESET}{NEWLINE}{COLOR_ST}❯{RESET} "
```
By convention, environment variables are ALL CAPS.

### Job Control Commands
`fg`, `bg` and `jobs` are job control commands. `fg` and `bg` are used to continue suspended jobs in the foreground and background respectively, while `jobs` is used to list currently suspended jobs.

See [job control](jobcontrol.md) for more details.

### Exit
Exits the shell.
Passing this command a number will cause the shell to exit with the specified number. If nothing is passed, the shell exits with exit code 0.

If non-numeric arguments are passed, the shell will print an error and exit with code 255.

### Cd
Changes the directory to the specified directory. Fails with exit code 1 if the path specified is not a directory, or if more than one argument is specified.

Oyster also features implicit `cd`. If it detects that the first word of the first command is a directory, it invokes `cd` on that directory.

### Let
`let` is the main method by which variables are set. The other way to do it is implicitly using the `<name>=<value>` notation. However, `let` is the only way to explicitly set the types of variables.
```
let str number = 2.4 (can be parsed to flt, but assigned as str)
let int number2 = 5 (parsed as int)
let number3 = 3.14 (inferred as flt)
let text = "hello" (inferred as str)
```
See [functions and expansions](expansions.md) for more information.

### Show
`show` allows the user to view the value of aliases, functions and variables. It has the optional flags `-f`, `-v` and `-a` for function, variable and alias respectively. If the flag is not specified, `show` will search in the order Functions, Variables, then Aliases, and return the first match.
```
$ let str greeting = "hey there!"
$ show -v greeting
str: "hey there!"

$ func greet 2
func > echo hello $greet0 and $greet1
func > echo nice to meet you
func > endfn

$ show -f greet
func greet 2
   echo hello $greet0 and $greet1 
   echo nice to meet you 
endfn
```

### Which
`which` allows the user to see the absolute path of each command, that is, the source program that is invoked for every command. `which` can detect shell builtin commands and reserved words, and inform the user accordingly.
```
$ which pacman
/usr/bin/pacman

$ which cogsy
/home/sammy/.cargo/bin/cogsy

$ which alias
alias: built in shell command

$ which else
else: shell reserved word
```
`which` can accept multiple arguments, and will evaluate each one accordingly.

Which returns an exit code equivalent to the number of arguments that did not match.

### To Be Implemented
The following commands have not been implemented, but will be.
- `read` reads a single line of input from the console and saves it to a variable.
- `type` tests the type of variable passed as an argument to it.
- `source` reads and executes a shell script without forking a new process.