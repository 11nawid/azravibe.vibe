## Variables & Types

Azra lets you store values using the `set` keyword. Types are inferred automatically.

### Declaring Variables
```vibe
set name to "Alice"   note: text
set age to 30         note: integer
set price to 12.99    note: float
set active to true    note: boolean
```
You can also use `#` or `/* ... */` comments. A trailing period at the end of a statement is ignored.

### Collections
- **List**: ordered collection, written as comma-separated values.
  ```vibe
  set numbers to 1, 2, 3
  add 4 to numbers
  remove 2 from numbers
  say numbers
  ```
  List items are read with `item 2 of numbers` (1-based) or changed with `set item 2 of numbers to 9`.
- **Map**: string-keyed collection, created with `empty map`.
  ```vibe
  set person to empty map
  set key "name" of person to "Bob"
  set key "age" of person to 25
  say key "name" of person
  say person has key "age"
  ```
  `person's "name"` is an equivalent shorthand for `key "name" of person`.

---
Proceed to the next lesson: **Control Flow**.
