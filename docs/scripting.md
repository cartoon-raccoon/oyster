## Scripting
Like any self-respecting shell, Oyster provides a system for creating shell scripts. Each block of code that the shell interprets as a loop or if statement is called a scripting construct. There are two kinds of constructs; control flow statements and loops. The parser can detect shell constructs and nested shell constructs, and will wait for the top level construct to be complete before executing it.

If currently inside a shell construct and the shell is waiting for input, it will display the prompt `<construct> > `.
```
$ for i in [1..5]
for > if [$i == 3]
if > while [$i > 5]
while > echo $i
while > done
if > end 
for > done

(this construct produces no output because the while test fails immediately.)
```

### Scripting Constructs
A scripting construct is a recursive data type represented by an enum in memory. It contains 3 variants: For, While, If and Code. All variants except Code contain another construct, effectively forming a recursive data type. This allows many constructs to be nested within a single construct, allowing for nested loops and if statements.

The recursive nature of the construct lends itself to forming an Abstract Syntax Tree, which the interpreter traverses preorder. The Code variants of the construct are the leaves (base case) of the AST, containing the raw commands to execute. (see the [source code](../src/scripting.rs) for details.)

The shell parses construct keywords like any other command and job, they are simply detected before the jobs are passed to the executor, and the jobs making up the construct are extracted and parsed separately, which is then executed.

Each command that makes up the scripting construct *must* be separated with consec delimiters; any other conditional execution token will result in an error. Using pipes will not cause errors, but can cause different effects depending on whether you write the construct on a single line or across multiple lines.

### Square Bracket Notation
Square brackets have special meaning to the construct interpreter, depending on which construct variant it is applied to. They can take the form of a range `[<integer>..<integer>]`, or equality evaluation `[$<variable> <equality operator> <some value>]`. Ranges are used in for loops, while equality evaluations are used in while loops and if statements.

**Range Expansions**

`[<integer>..=<integer>..<integer>]`

This expands to a list of strings from the first integer, stepping over by the value of the third integer, up until just before the second integer. The third integer is optional, and if not specified, the range will increment/decrement by 1. If the first integer is lesser than the second, the range will increment, else it will decrement.

The third integer must be positive; the shell will throw an error if it is not.

e.g. `[1..5]` expands to `1 2 3 4`.

To include the second integer, use `[<integer>..=<integer>]`.

e.g. `[1..=5]` expands to `1 2 3 4 5`.

`[0..=10..2]` expands to `0 2 4 6 8 10`.

`=` does not respect step over, and will end the range on the second integer.

e.g. `[1..=10..2]` expands to `1 3 5 7 9 10`.

**Equality Evaluations**

`[<some value> <equality operator> <some value>]`

Only variables of the same type can be compared. The shell will throw an error otherwise.

The accepted equality operators are:
- `-eq` or `==` - Equal to
- `-ne` or `!=` - Not equal
- `-lt` or `< ` - Less than
- `-gt` or `> ` - Greater than
- `-le` or `<=` - Less than or equal to
- `-ge` or `>=` - Greater than or equal to

You can put variables on either side, expanding them as usual with `$`.
String variables are compared lexicographically.

e.g.
```
$ let int number = 2
$ show -v number
int: 2

$ if [$number == 2];
if > echo cool this worked
if > else
if > echo this did not work
if > end

cool this worked
```
Additionally (and probably less usefully), you can make a evaluation automatically evaluate to true or false simply by putting the corresponding value inside the bracket, e.g. `[true]` or `[false]`. This is mostly useful for creating infinite loops with `while [true]`.

This is better than using `[$i == $i]` because the latter notation requires the shell to tokenize and parse the enclosed text, expand the variables and then test for equality, whereas there is an explicit check for `true` or `false` and the evaluation can immediately return the corresponding value.

### If Statements
The general syntax of the if statement is as follows:

`if <condition>; <execute commands here>; end`

The `end` keyword denotes the end of the if statement.

In the middle you can add additional branches with `elif` and `else`:
```
if <condition>;
<execute commands>
elif <condition>;
<execute commands>
else;
<execute commands>
end
```
The condition can be a square bracket construct, or a command; the test succeeds if the command's exit status is 0, otherwise it fails and control moves to the next branch if any.
```
$ if cat ~/Documents 2>/dev/null
if > echo "this shouldn't work"
if > elif echo hello &> /dev/null
if > echo "this branch gets executed"
if > else
if > echo "this branch gets skipped"
if > end

this branch gets executed
```
The only accepted square bracket notation here is equality evaluation.

You can also use command substitution to test for the output of a command. Both sides of the equality operator get parsed to Str variables only; no type inference here.
```
$ if $(echo hello) == "hello"
if > echo howdy
if > end
howdy
```
This is useful if you need to test an information file in `/sys` for a kernel status, or something similar.

### For Loops
The syntax of the for loop is as follows:

`for <variable> in <loop-over>; <execute commands here>; done`

The done keyword is very important as it denotes the end of the loop. Without it, the shell will loop over every subsequent command until the done keyword is specified.

The loop-over construct is simply the arguments to the for keyword following `in`, which can be expanded in different ways.

The simplest loop-over is a simple list:

`for i in 1 2 3 4 5;`

The only square bracket notation here is the range notation:

Brace and glob expansions as well as command substitution are also valid in for loops. They expand to a list of strings accordingly.

The variable in the loop declaration is a valid shell variable, and can be expanded:
```
$ for name in 1{0,1,2,3,4}
for > echo $name
for > done

10
11
12
13
14
```
However, the variable is only valid for the duration of the loop, and will be removed from the shell once the loop ends. Any subsequent attempts to expand the variable before it is re-defined will result in an empty string.

### While Loops
Oyster can also execute while loops; while a condition evaluates to true, do the code enclosed within. Similar to if statements, this can be a command or a square bracket containing equality evaluation notation. See the section on if statements for the full notation.
```
$ while [$i > 0]
while > echo $i
while > let i = [$i - 1]
while > done

10
9
8
7
6
5
4
3
2
1

$ while cat ~/Documents
while > echo "this shouldn't work"
while > done

cat: /home/sammy/Documents: Is a directory
```

### Running scripts
Oyster can also execute script files. When invoked, it checks its second argument, and if it exists, it opens the file specified there and executes it.

`oyster /path/to/script`

It reads the file line by line, skipping any empty lines are lines beginning with `#` (comments).
Any time it encounters a new line, it treats the command as a new job.
Note that the shell executes scripts as a separate process with a separate address space, and any variables in the current shell session will not exist in the script execution. To run a script in the current shell session, use the `source` command.

`source /path/to/script`

Like bash or zsh, you can include the shebang `#!path/to/executable` on the very first line to execute the script like a command.

Note: Always put comments on their own newline. Comments on the same line as code break the parser for some reason. This is a bug and will be fixed.
