## Functions and Variables
Oyster provides various ways to manipulate data within the shell itself. The shell itself is a programming language, so it implements most of the features you would expect from a programming language, such as control flow, loops, and covered here, functions and variables.

To look at the other programming constructs, see [Scripting Constructs](scripting.md).

### Functions
Functions are simply blocks of commands assigned to a common identifier, and are run together, in the sequence specified. While similar to aliases. functions differ in that like real programming functions, they can be passed parameters. They are also more versatile than aliases, in that they can easily contain scripting constructs, and can be used to run multiple scripting constructs on demand with a single word.

Functions are defined with the `func` keyword, and are called with the name and `()` concatenated to the back.
To denote the end of the function, you must use the `endfn` keyword.
As with scripting constructs, the shell parser can detect functions, and will wait for further input if it detects the function is not yet complete.
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
However, scripting constructs can be defined inside functions:
```
$ func thing2
func > for i in [1..=10]
for > if [$i < 6]
if > echo "less than 6: $i"
if > else
if > echo "more than 5: $i"
if > end
for > done
func > endfn

$ thing2()
less than 6: 1
less than 6: 2
less than 6: 3
less than 6: 4
less than 6: 5
more than 5: 6
more than 5: 7
more than 5: 8
more than 5: 9
more than 5: 10
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
When the function is run, the shell adds the parameters as variables under the naming convention mentioned above. If a variable with that name already exists, it will be overridden. Once the function ends, the shell deletes the variables from its memory.

If the number of parameters passed and the number of parameters specified do not match, the function will return an error. Functions defined without a parameter count are automatically variadic and can accept any number of functions. If there are more variables specified in the function body than parameters passed, the missing variables will expand to empty strings.

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

As such, recursive functions can be defined in Oyster. To prevent stack overflows in the shell itself, Oyster implements an internal stack that grows every time a function is called, and shrinks when a function ends. Oyster also has a maximum nesting depth that is checked every time a function is called. If this depth is exceeded by the stack, Oyster will automatically return an error.

The following function can calculate the factorial of a number up to 20 (at which point the variable overflows):
```
func fact 1
    if [$fact0 <= 1]
        echo 1
    else
        let int temp = 1
        for i in [$fact0..=1]
            let int temp = [$temp * $i]
        done
        echo $temp
    end
endfn
```
Recursive functions can be defined and called in Oyster, but they are still very wonky and won't be helpful the vast majority of the time. It is best to stick to an iterative approach to scripting. This applies to most shell scripting languages.

### Variables
The shell can also accept user-defined variables. Variables can take one of three main types: Str (string), Int (integer) and Flt (float).

There are two ways to define variables: implicitly, and with the `let` command.
```
<name>=<value>
let <type> <name> = <value>
```
The second way is the only way to assign quoted text. Variable types are mostly inferred: the first way will automatically infer types, and `let` will infer types if `<type>` is not specified.

Ints are stored internally as signed 64-bit numbers, and floats are stored as signed 64-bit floats. Oyster can detect overflow or underflow when performing operations, and will return an error if this happens. If attempting to assign a value greater than the maximum value of the variable type, `let` will return an error if the type is specified, if not it will follow type inference procedure and assign the variable as the type that first passes the parse.

Variables can only accept alphanumeric names. Implicit declaration will fail the check if non-alphanumeric characters are in the variable name, and the word will be executed as a command:
```
$ hello!=2
oyster: command hello!=2 not found
```
`let` will do an explicit check for this constraint and return an error if it is not met.

_Note:_ using quotes with let will not affect the way the value is inferred. `let numstring = "2"` will still yield an Int of value 2. This is because `let` is run and passed its arguments after almost all parsing and expansions have been completed, so the quotes by now will have been removed, which means `let` cannot see a difference between `2` and `"2"`. To explicitly pass a number as a string, you need to specify the type as an argument to `let`:

`let str numstring = 2`

Type inference works as follows: The shell first attempts to parse the value as an int. If it fails, the shell then tries to parse it as a float. If that too fails, the shell parses the value as a string.
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
Expanding variables always expands to a string. To operate on variables as their types, you need to enclose the operation inside a square bracket. See [expansions](expansions.md) for more information.

As of now, `$` cannot be backslash-escaped. The only way to use a literal $ is to enclose it in single quotes (variable expansion is performed on double quotes). This is a bug and will be fixed.