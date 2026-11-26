## Modules & Imports

Large projects can be split into multiple files. Use `use "filename.azra"` to load a module.

### Creating a Module
Create a file `math.azra`:
```azra
define add a b:
    give back a + b

define sub a b:
    give back a - b
```

### Using a Module
In your main script:
```azra
use "math.azra"
set result to add 10 5
say "Sum = " + result
set result2 to sub 10 5
say "Diff = " + result2
```

Modules can also contain variables and other functions.

---
Proceed to the next lesson: **Full Example Project**.
