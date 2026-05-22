# Shell `if [ ... ]` Tests

In shell scripts, `if [ ... ]; then` uses `test` expressions. These expressions can check files, directories, strings, and numbers.

## File And Directory Checks

```sh
if [ -d "$path" ]; then
```

`-d` is true when `$path` exists and is a directory.

```sh
if [ -f "$path" ]; then
```

`-f` is true when `$path` exists and is a regular file.

For a Snap package output like:

```sh
"$OUTPUT_DIR/$OUTPUT"
```

`-f` is the right check because the expected result is a `.snap` file. `-d` would only be true if there were a directory with that exact `.snap` name.

## Common File Checks

| Test | Meaning |
| --- | --- |
| `-e "$path"` | Exists, any type |
| `-f "$path"` | Exists and is a regular file |
| `-d "$path"` | Exists and is a directory |
| `-L "$path"` | Exists and is a symbolic link |
| `-r "$path"` | Exists and is readable |
| `-w "$path"` | Exists and is writable |
| `-x "$path"` | Exists and is executable or searchable |
| `-s "$path"` | Exists and has a size greater than zero |

## Common String Checks

| Test | Meaning |
| --- | --- |
| `-z "$var"` | String is empty |
| `-n "$var"` | String is not empty |
| `"$a" = "$b"` | Strings are equal |
| `"$a" != "$b"` | Strings are different |

## Common Number Checks

| Test | Meaning |
| --- | --- |
| `"$a" -eq "$b"` | Equal |
| `"$a" -ne "$b"` | Not equal |
| `"$a" -lt "$b"` | Less than |
| `"$a" -le "$b"` | Less than or equal |
| `"$a" -gt "$b"` | Greater than |
| `"$a" -ge "$b"` | Greater than or equal |

## Quoting Variables

Always quote variables inside `[ ... ]`:

```sh
if [ -f "$path" ]; then
```

Quoting protects paths with spaces and avoids errors when a variable is empty.
