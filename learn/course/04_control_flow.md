## Control Flow

Control structures let you direct the execution of your program. Blocks always start with `:` and are indented by 4 spaces.

### If / Otherwise
```vibe
set age to 20
if age greater_than= 18:
    say "Adult"
otherwise:
    say "Minor"
```
Extra branches use `otherwise if`:
```vibe
if age less_than 13:
    say "Child"
otherwise if age less_than 18:
    say "Teen"
otherwise:
    say "Adult"
```
Available comparisons are `equals`, `not_equals`, `less_than`, `less_than=`, `greater_than`, `greater_than=`, plus `is`, `is not`, `contains`, `starts with` and `has key`. Symbolic `>=`, `<`, `>` and `!=` are not part of Azra.

### While Loop
```vibe
set counter to 0
while counter less_than 5:
    say counter
    set counter to counter + 1
```

### For Loop (over a list)
```vibe
set items to "a", "b", "c"
for each item in items:
    say item
```
`repeat 3 times:` repeats a fixed number of times, and `repeat with i from 1 to 10 by 2:` counts over a range. Use `stop loop` / `break` and `skip next` / `continue` inside loops.

---
Proceed to the next lesson: **Functions**.
