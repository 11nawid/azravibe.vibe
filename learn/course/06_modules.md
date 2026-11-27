## Modules & Imports

Large projects can be split into multiple files. Use `use "filename.vibe"` to load a module; an optional `as alias` gives it a short name.

### Creating a Module
Create a file `math.vibe`:
```vibe
define add with a, b:
    give back a + b

define sub with a, b:
    give back a - b
```

### Using a Module
In your main script:
```vibe
use "math.vibe"
set result to add with 10, 5
say "Sum = " + result
set result2 to sub with 10, 5
say "Diff = " + result2
```
Functions defined in a module are added to the importing script's globals. Modules can also contain variables, objects and event handlers. Import cycles are detected and reported as errors.

---
Proceed to the next lesson: **Full Example Project**.
