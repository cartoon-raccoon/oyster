## Scripting
Like any self-respecting shell, Oyster provides a system for creating shell scripts. Each block of code that the shell interprets as a loop or if statement is called a scripting construct. There are two kinds of constructs; the if statement and the for loop.

### Scripting Constructs
A scripting construct is a recursive data type represented by an enum in memory. It contains 3 variants: For, If and Code. The For and If variants contain another construct, effectively forming a recursive data type. This allows many constructs to be nested within a single construct, allowing for nested loops and if statements.

The recursive nature of the construct lends itself to forming an Abstract Syntax Tree, which the interpreter traverses preorder. The Code variants of the construct are the leaves of the AST, containing the raw commands to execute. (see the [source code](src/scripting.rs) for details.)

The shell parses construct keywords like any other command and job, they are simply detected before the jobs are passed to the executor, and the jobs making up the construct are extracted and parsed separately, which is then executed.

### Square Brackets
Square brackets have special meaning to the interpreter, depending on which construct variant it is applied to. They can take the form of a range `[<integer>..<integer>]`, or equality evaluation `[$<variable> <equality operator> <some value>]`.

### For Loops
The syntax of the for loop is as follows:

`for <variable> in <loop-over>; <execute commands here>; done`

The done keyword is very important as it denotes the end of the loop. Each command *must* be separated with consec delimiters; any other conditonal execution token will result in an error.

The loop-over construct is simply a list of arguments to the for keyword, which can be expanded in different ways.

The simplest loop-over is a simple list:

`for i in 1 2 3 4 5;`

The only square bracket notation here is the range notation:

`[<integer>..<integer>]`

This expands to a list of strings from the first integer, incrementing by 1, up until just before the second integer.

e.g. `[1..5]` expands to `1 2 3 4`.

To include the second integer, use `[<integer>..=<integer>]`.

e.g. `[1..=5]` expands to `1 2 3 4 5`.

Brace and glob expansions are also valid here.

The variable in the loop declaration is a valid shell variable, and can be expanded:
```
$ for name in 1{0,1,2,3,4}
For > echo $name
For > done

10
11
12
13
14
```

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
If > echo "this shouldn't work"
If > elif echo hello &> /dev/null
If > echo "this branch gets executed"
If > else
If > echo "this branch gets skipped"
If > end

this branch gets executed
```
The only accepted square bracket notation here is equality evaluation:

`[$<variable> <equality operator> <some value>]`

This requires variable typing to be implemented, and thus has not been implemented yet.

Only variables of the same type can be compared. The shell will throw an error otherwise.

The accepted equality operators are:
- `-eq` - Equal to
- `-ne` - Not equal
- `-lt` - Less than
- `-gt` - Greater than
- `-le` - Less than or equal to
- `-ge` - Greater than or equal to

### Running scripts
Oyster can also execute script files. When invoked, it checks its second argument, and if it exists, it opens the file specified there and executes it.

It reads the file line by line, skipping any empty lines are lines beginning with `#` (comments).
Any time it encounters a new line, it treats the command as a new job.

Like bash or zsh, you can include the shebang `#!path/to/executable` on the very first line to execute the script like a command.

Note: Always put comments on their own newline. Comments on the same line as code break the parser for some reason. This is a bug and will be fixed.