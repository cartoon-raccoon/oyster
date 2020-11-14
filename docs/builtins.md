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

### Pushd and Popd
Oyster also maintains a directory stack which is simply a list of recently visited directories. It could prove useful to have if you are constantly switching between directories.

`pushd` and `popd` are the two commands that can manipulate the directory stack. `pushd` pushes directories to the stack, while `popd` removes directories from the stack. The elements on the stack are numbered from 0, from left to right.

Invoking either command also triggers a `cd` to the directory that was manipulated on the stack. Passing the `-n` switch after the command will prevent the directory change.

`pushd` can accept either an integer or pathname. If a pathname is given, it will push the path to the top of the stack. If no arguments are passed, it swaps the top two elements on the stack and changes to the top directory, depending on whether the `-n` flag is set.

The integer `N` passed can be positive or negative. If positive, the `N`th path from the left is brought to the top, otherwise the `N`th from the right is moved.

`popd` only accepts an integer. Following the same rules for integer `N`, it removes the `N`th integer from the stack and changes to it, depending on whether the `-n` flag is passed.

### Dirs
The `dirs` command complements the dirstack manipulation commands, by allowing the user to view the stack. It accepts the `-clpv` flags and an integer `N` in the same fashion as `pushd` and `popd`.

- `-c` clears the directory stack.
- `-l` shows the full path of each item; `dirs` uses tilde notation by default.
- `-p` causes dirs to print each entry on a newline.
- `-v` causes dirs to print each entry on a newline, prefixing each path with its index in the stack.

`+N` shows the `N`th integer from the left, and `-N` the `N`th integer from the right.

### To Be Implemented
The following commands have not been implemented, but will be.
- `read` reads a single line of input from the console and saves it to a variable.
- `type` tests the type of variable passed as an argument to it.
- `source` reads and executes a shell script without forking a new process.