## Functions

Functions let you encapsulate reusable logic. Like Azra itself, they never use parentheses.

### Defining a Function
```vibe
define greet with name:
    say "Hello, " + name
```

### Calling a Function
```vibe
greet with "Bob"
```
Calls in expressions also use `with`:
```vibe
set message to greet with "Alice"
```

### Returning Values
Use `give back` to return a value.
```vibe
define add with a, b:
    give back a + b

set sum to add with 3, 5
say sum   # prints 8
```
A function with no parameters runs when its name is used in an expression:
```vibe
define status:
    give back "all systems go"
say status
```

---
Proceed to the next lesson: **Modules & Imports**.
