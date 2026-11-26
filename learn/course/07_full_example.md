## Full Example Project

In this final lesson we combine everything we have learned to build a simple calculator program.

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

### `calc.azra`
Create a file `calc.azra` with the following content:
```azra
# Import math helpers
use "math.azra"

say "Enter first number:"
set a to read number
say "Enter second number:"
set b to read number

say "Sum: " + add a b
say "Difference: " + sub a b
say "Product: " + mul a b
if b != 0:
    say "Quotient: " + div a b
else:
    say "Cannot divide by zero"
```

### `math.azra`
```azra
define add a b:
    give back a + b

define sub a b:
    give back a - b

define mul a b:
    give back a * b

define div a b:
    give back a / b
```

### Running the Program
```bash
azra calc.azra
```
You will be prompted for two numbers and see the calculated results.

---
Congratulations! You have completed the Azra language course.
