## Basic Syntax

Azra programs consist of statements, each ending with a newline. Indentation defines blocks: a header ends with `:` and its body is indented by exactly 4 spaces. Tabs are not allowed.

### Hello World
```vibe
say "Hello, world!"
```

### Comments
Use `note:` or `#` for line comments, and `/* ... */` for block comments.
```vibe
note: This is a comment
# This is also a comment
/* This block
   spans lines */
say "Hi"
```
A comment-only line does not change the indentation of a block.

### Numbers, strings and booleans
```vibe
set count to 3
set price to 12.99
set name to "Alice"
set active to true
```
Integer division truncates: `say 7 / 2` prints `3`. Use a decimal operand for a fractional result: `say 7 / 2.0` prints `3.5`.

---
Proceed to the next lesson: **Variables & Types**.
