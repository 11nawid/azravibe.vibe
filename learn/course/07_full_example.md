## Full Example Project

In this final lesson we combine everything we have learned to build a small calculator script backed by a module.

> Note: this interpreter has no keyboard-input primitive. The example uses
> predefined values so it can run unchanged from the command line.

### Project Structure
```
learn/course/
├─ 01_introduction.md
├─ 02_basics.md
├─ 03_variables.md
├─ 04_control_flow.md
├─ 05_functions.md
├─ 06_modules.md
└─ 07_full_example.md
```

### `calc.vibe`
Create a file `calc.vibe` with the following content:
```vibe
note: import math helpers
use "math.vibe"

set a to 8
set b to 2

say "Sum: " + add with a, b
say "Difference: " + sub with a, b
say "Product: " + mul with a, b
if b not_equals 0:
    say "Quotient: " + div with a, b
otherwise:
    say "Cannot divide by zero"
```

### `math.vibe`
```vibe
define add with a, b:
    give back a + b

define sub with a, b:
    give back a - b

define mul with a, b:
    give back a * b

define div with a, b:
    give back a / b
```

### Running the Program
```bash
azra run calc.vibe
```
Output:
```
Sum: 10
Difference: 6
Product: 16
Quotient: 4
```

---
Congratulations! You have completed the Azra language course.
