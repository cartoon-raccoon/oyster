## Prompt
The prompt is completely customizable. As of now, the prompt can display the hostname, the current user's name, and the current working directory.

Oyster configures its prompt through the `OYSTER_PROMPT` environment variable. This can be exported in its RC file.

To define the prompt, you can use keywords enclosed in braces `{}`.

The currently accepted keywords are:
- `CWD_FULL`: Renders the entire path of the present working directory. If pwd is a subdirectory of $HOME, it shortens to tilde notation.
- `CWD_TOP`: Renders the top level of the present working directory.
- `USER`: Displays the username.
- `HOST`: Displays the hostname.
- `GIT_REPO`: If the pwd is a git repository, it displays the current working branch. (To be implemented)
- `NEWLINE`: Continues the prompt on a new line.
- `COLOR_ST`: Changes colour depending on the last exit status of the last job. If 0, it displays green, else it displays red.

The currently accepted colours are blue, yellow, black, white, red and green.
They also have `_B` variants that display their bold variants.
The `RESET` keyword must be used to return the colours back to white.
To use the literal brace, you can escape it with a backslash.

For example, `{YELLOW_B}[{HOST}] {USER}{RESET}: {BLUE}{CWD_FULL}{RESET}{NEWLINE}{COLOR_ST}❯{RESET} ` generates the following prompt:

```
[<hostname>] <username>: <pwd>
❯
```

The default prompt is `{BOLD}{USER}{RESET}: {CWD_TOP} {COLOR_ST}>>{RESET} ` which Oyster uses if `OYSTER_PROMPT` is not present.