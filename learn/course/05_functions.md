## Functions

Functions let you encapsulate reusable logic.

### Defining a Function
```azra
define greet name:
    say "Hello, " + name
```

### Calling a Function
```azra
greet "Bob"
```

### Returning Values
Use `give back` to return a value.
```azra
define add a b:
    give back a + b

set sum to add 3 5
say sum   # prints 8
```

---
Proceed to the next lesson: **Modules & Imports**.
