# CoVibe Language Specification
## Part 2: Control Flow, Functions, and Pattern Matching Grammar

**Version:** 1.0
**Date:** 2026-04-14
**Status:** Final

---

## Table of Contents

1. [Introduction](#introduction)
2. [Control Flow Constructs](#control-flow-constructs)
3. [Function Declarations](#function-declarations)
4. [Pattern Matching](#pattern-matching)
5. [Lambda Expressions and Closures](#lambda-expressions-and-closures)
6. [Generator Functions](#generator-functions)
7. [Comprehensions](#comprehensions)
8. [Decorator and Annotation System](#decorator-and-annotation-system)

---

## 1. Introduction

This document specifies the control flow constructs, function declaration and invocation semantics, pattern matching with exhaustiveness checking, lambda expressions, closures, generators, comprehensions, and the decorator system in the CoVibe programming language.

This specification builds upon Part 1 (Lexical Structure and Core Syntax Grammar) and assumes familiarity with the basic syntax and token definitions established there.

---

## 2. Control Flow Constructs

CoVibe provides a comprehensive set of control flow constructs that balance simplicity with expressiveness.

### 2.1 Conditional Statements: if/elif/else

#### 2.1.1 Grammar

```ebnf
IfStatement =
    'if' Expression ':' Block
    ('elif' Expression ':' Block)*
    ('else' ':' Block)?

IfExpression =
    'if' Expression ':' (Expression | Block)
    ('elif' Expression ':' (Expression | Block))*
    ('else' ':' (Expression | Block))?
```

#### 2.1.2 Semantics

**Type Requirements:**
- The condition expression in `if` and `elif` must have type `bool`.
- If used as an expression, all branches must evaluate to the same type or compatible types that can be unified.
- If any branch is missing and the construct is used as an expression, the type must be `Option<T>` where `T` is the branch result type.

**Evaluation:**
1. The condition of the `if` clause is evaluated.
2. If true, the corresponding block is executed, and no other clauses are evaluated.
3. If false, each `elif` clause is evaluated in order until one evaluates to true.
4. If no condition is true and an `else` clause exists, it is executed.
5. If no condition is true and no `else` clause exists, the statement produces the unit value `()`.

**Expression Form:**
When used as an expression, the if construct must produce a value from all branches:

```covibe
let x = if condition: 42 else: 0
let y = if a > b: a elif a < b: b else: 0
```

If an `else` clause is omitted in expression context, the type must be `Option<T>`:

```covibe
let x: Option<int> = if flag: 42  # Type is Option<int>
```

#### 2.1.3 Short-Circuit Evaluation

Conditions in `if` and `elif` use short-circuit evaluation for logical operators:
- `a and b` — if `a` is false, `b` is not evaluated
- `a or b` — if `a` is true, `b` is not evaluated

#### 2.1.4 Examples

```covibe
# Statement form
if x > 0:
    print("positive")
elif x < 0:
    print("negative")
else:
    print("zero")

# Expression form
let abs_x = if x >= 0: x else: -x

# Nested conditions
if is_valid(input):
    if has_permission(user):
        process(input)
    else:
        raise PermissionError()
else:
    raise ValidationError()
```

---

### 2.2 While Loops

#### 2.2.1 Grammar

```ebnf
WhileStatement = 'while' Expression ':' Block
```

#### 2.2.2 Semantics

**Type Requirements:**
- The condition expression must have type `bool`.

**Evaluation:**
1. The condition is evaluated.
2. If true, the block is executed, and control returns to step 1.
3. If false, control exits the loop.

**Break and Continue:**
- `break` — immediately exits the loop
- `continue` — skips to the next iteration

**Loop Result:**
While loops evaluate to `()` by default. If a `break` statement provides an expression, that becomes the loop result:

```covibe
let result = while condition:
    if found:
        break value
    # ... continue processing
# result has type Option<T> where T is the type of value
```

#### 2.2.3 Examples

```covibe
# Basic while loop
var i = 0
while i < 10:
    print(i)
    i += 1

# While with break
while true:
    let input = read_line()
    if input == "quit":
        break
    process(input)

# While with continue
var sum = 0
var i = 0
while i < 100:
    i += 1
    if i % 2 == 0:
        continue
    sum += i
```

---

### 2.3 For Loops

#### 2.3.1 Grammar

```ebnf
ForStatement = 'for' Pattern 'in' Expression ':' Block
```

#### 2.3.2 Semantics

**Type Requirements:**
- The expression after `in` must implement the `Iterator` trait or be convertible to an iterator via `into_iter()`.
- The pattern must match the item type produced by the iterator.

**Evaluation:**
1. The iterator expression is evaluated once, producing an iterator.
2. For each item yielded by the iterator:
   a. The item is matched against the pattern.
   b. Pattern bindings are introduced into the loop body scope.
   c. The loop body is executed.
3. When the iterator is exhausted, the loop terminates.

**Destructuring in For Loops:**

Patterns in for loops support full destructuring:

```covibe
for (key, value) in map:
    print(f"{key}: {value}")

for User { name, age } in users:
    print(f"{name} is {age} years old")
```

**Break and Continue:**
Same semantics as while loops.

#### 2.3.3 Examples

```covibe
# Iterate over range
for i in 0..10:
    print(i)

# Iterate over collection
for item in collection:
    process(item)

# Enumerate
for (index, value) in collection.enumerate():
    print(f"[{index}] {value}")

# Destructuring
for (x, y, z) in points:
    let distance = (x**2 + y**2 + z**2).sqrt()
    print(distance)

# Filtering
for x in numbers:
    if x % 2 == 0:
        continue
    print(x)
```

---

### 2.4 Infinite Loops

#### 2.4.1 Grammar

```ebnf
LoopStatement = 'loop' ':' Block
```

#### 2.4.2 Semantics

The `loop` construct creates an infinite loop that must be explicitly exited using `break` or `return`.

**Type:**
- Without `break expr`, type is `!` (never type)
- With `break expr`, type is the type of the break expression

**Examples:**

```covibe
# Infinite loop with explicit exit
loop:
    let input = read_line()
    if input == "quit":
        break
    process(input)

# Loop with value
let result = loop:
    let x = compute()
    if is_valid(x):
        break x
# result has type T where T is the type of x
```

---

### 2.5 Break and Continue

#### 2.5.1 Grammar

```ebnf
BreakStatement    = 'break' Expression? NEWLINE
ContinueStatement = 'continue' NEWLINE
```

#### 2.5.2 Semantics

**Break:**
- Exits the innermost enclosing loop (`for`, `while`, `loop`).
- Can optionally provide a value that becomes the loop result.
- If used inside nested loops, only the innermost loop is exited.

**Continue:**
- Skips the remainder of the current loop iteration.
- Control returns to the loop condition (for `while`) or the next iterator item (for `for`).

**Loop Labels:**

Loops can be labeled to allow breaking or continuing outer loops:

```ebnf
LabeledLoop = Lifetime (':' (ForStatement | WhileStatement | LoopStatement))
Lifetime    = "'" Identifier

BreakStatement    = 'break' Lifetime? Expression? NEWLINE
ContinueStatement = 'continue' Lifetime? NEWLINE
```

Examples:

```covibe
'outer: for i in 0..10:
    'inner: for j in 0..10:
        if condition:
            break 'outer  # Exits both loops
        if other_condition:
            continue 'outer  # Continue outer loop
```

---

### 2.6 Return Statement

#### 2.6.1 Grammar

```ebnf
ReturnStatement = 'return' (Expression (',' Expression)*)? NEWLINE
```

#### 2.6.2 Semantics

**Single Value Return:**
```covibe
return 42
return compute_value()
```

**Multiple Value Return:**

CoVibe supports returning multiple values without explicit tuple wrapping:

```covibe
def divide_with_remainder(a: int, b: int) -> (int, int):
    return a // b, a % b

let (quotient, remainder) = divide_with_remainder(17, 5)
```

This is syntactic sugar for returning a tuple `(a // b, a % b)`.

**Early Return:**

Return statements can appear anywhere in a function body:

```covibe
def find(items: [int], target: int) -> Option<int>:
    for (i, item) in items.enumerate():
        if item == target:
            return Some(i)
    return None
```

**Type Checking:**
- The return type must match the function's declared return type.
- If no return type is declared, it is inferred from all return statements.
- All code paths must return a value, or the function must return `()`.

---

## 3. Function Declarations

### 3.1 Function Declaration Grammar

```ebnf
FunctionDeclaration =
    Decorator* Visibility? 'async'? 'def' Identifier
    GenericParams? '(' FunctionParams? ')' ('->' TypeExpression)?
    WhereClause?
    (':' Block | '=' Expression NEWLINE)

Visibility = 'pub' | 'priv' | 'protected'

GenericParams = '<' GenericParam (',' GenericParam)* ','? '>'

GenericParam =
    | Identifier (':' TypeBound)?
    | 'const' Identifier ':' TypeExpression
    | Lifetime (':' LifetimeBound)?

TypeBound = TypeExpression ('+' TypeExpression)*

LifetimeBound = Lifetime ('+' Lifetime)*

FunctionParams = FunctionParam (',' FunctionParam)* ','?

FunctionParam =
    | Pattern ':' TypeExpression ('=' Expression)?
    | 'self'
    | 'mut' 'self'
    | '&' 'self'
    | '&' 'mut' 'self'
    | '...' Identifier

WhereClause = 'where' WhereConstraint (',' WhereConstraint)* ','?

WhereConstraint = TypeExpression ':' TypeBound
```

### 3.2 Function Components

#### 3.2.1 Visibility

Functions can have three visibility levels:
- `pub` — public, visible outside the module
- `priv` — private, visible only within the module (default if omitted)
- `protected` — visible within the module and submodules

```covibe
pub def public_function():
    # ...

priv def private_function():
    # ...

def default_is_private():
    # ...
```

#### 3.2.2 Generic Parameters

Functions can be generic over types, const values, and lifetimes:

```covibe
# Generic over type
def identity<T>(x: T) -> T:
    return x

# Generic with trait bound
def print_all<T: Display>(items: [T]):
    for item in items:
        print(item)

# Const generic
def create_array<const N: usize, T>(value: T) -> [T; N]:
    return [value; N]

# Lifetime generic
def longest<'a>(s1: &'a str, s2: &'a str) -> &'a str:
    if s1.len() > s2.len(): s1 else: s2
```

#### 3.2.3 Where Clauses

Complex trait bounds can be expressed using `where` clauses:

```covibe
def complex_function<T, U>(t: T, u: U) -> T
where
    T: Clone + Display,
    U: Into<T>:
    let converted: T = u.into()
    return t.clone()
```

#### 3.2.4 Function Parameters

**Named Parameters:**
```covibe
def greet(name: str, age: int):
    print(f"Hello {name}, you are {age} years old")
```

**Default Arguments:**
```covibe
def connect(host: str, port: int = 8080, timeout: float = 5.0):
    # ...

connect("localhost")  # Uses default port and timeout
connect("localhost", port=9000)  # Named argument
connect("localhost", 3000, 10.0)  # Positional arguments
```

**Self Parameters (for methods):**
```covibe
struct Counter:
    value: int

impl Counter:
    def new() -> Self:
        return Counter { value: 0 }

    def increment(&mut self):
        self.value += 1

    def get(&self) -> int:
        return self.value

    def consume(self) -> int:
        return self.value
```

**Variadic Parameters:**
```covibe
def sum_all(...numbers: int) -> int:
    var total = 0
    for n in numbers:
        total += n
    return total

sum_all(1, 2, 3, 4, 5)  # Returns 15
```

#### 3.2.5 Return Types

**Explicit Return Type:**
```covibe
def add(a: int, b: int) -> int:
    return a + b
```

**Inferred Return Type:**
```covibe
def add(a: int, b: int):
    return a + b  # Return type inferred as int
```

**Multiple Return Values:**
```covibe
def min_max(numbers: [int]) -> (int, int):
    return numbers.min(), numbers.max()
```

**Unit Return Type:**
Functions without a return statement or explicit return type return `()`:
```covibe
def log(message: str):
    print(message)  # Implicitly returns ()
```

#### 3.2.6 Expression Body Functions

Functions with a single expression can use the `=` syntax:

```covibe
def square(x: int) -> int = x * x

def is_even(n: int) -> bool = n % 2 == 0

def greet(name: str) = print(f"Hello, {name}!")
```

### 3.3 Function Invocation

#### 3.3.1 Positional Arguments

```covibe
def add(a: int, b: int) -> int:
    return a + b

add(5, 3)  # Returns 8
```

#### 3.3.2 Named Arguments

```covibe
def create_user(name: str, age: int, email: str):
    # ...

create_user(name="Alice", age=30, email="alice@example.com")
create_user("Alice", email="alice@example.com", age=30)
```

#### 3.3.3 Argument Unpacking

```covibe
def point(x: int, y: int, z: int):
    # ...

let coords = (10, 20, 30)
point(...coords)  # Unpacks tuple as positional arguments

let kwargs = { x: 10, y: 20, z: 30 }
point(...kwargs)  # Unpacks map as named arguments
```

### 3.4 Async Functions

```ebnf
AsyncFunctionDeclaration =
    Decorator* Visibility? 'async' 'def' Identifier
    GenericParams? '(' FunctionParams? ')' ('->' TypeExpression)?
    (':' Block | '=' Expression NEWLINE)
```

Async functions return a `Future<T>` that must be awaited:

```covibe
async def fetch_data(url: str) -> str:
    let response = await http.get(url)
    return await response.text()

# Usage
let data = await fetch_data("https://example.com")
```

---

## 4. Pattern Matching

### 4.1 Match Expression Grammar

```ebnf
MatchExpression =
    'match' Expression ':'?
    INDENT
    MatchArm+
    DEDENT

MatchStatement =
    'match' Expression ':'
    INDENT
    MatchArm+
    DEDENT

MatchArm =
    'case' Pattern Guard? '=>' (Expression | Block)

Guard = 'if' Expression
```

### 4.2 Pattern Grammar

```ebnf
Pattern =
    | LiteralPattern
    | IdentifierPattern
    | WildcardPattern
    | TuplePattern
    | StructPattern
    | EnumPattern
    | OrPattern
    | RangePattern
    | ReferencePattern
    | BoxPattern
    | ArrayPattern
    | RestPattern

LiteralPattern = Literal

IdentifierPattern = 'mut'? Identifier ('@' Pattern)?

WildcardPattern = '_'

TuplePattern = '(' (Pattern (',' Pattern)* ','?)? ')'

StructPattern =
    TypePath '{' (FieldPattern (',' FieldPattern)* ','? (',' '..')?)? '}'

FieldPattern =
    | Identifier (':' Pattern)?
    | '..'

EnumPattern =
    TypePath ('(' (Pattern (',' Pattern)* ','?)? ')')?

OrPattern = Pattern ('|' Pattern)+

RangePattern = Literal ('..' | '..=') Literal

ReferencePattern = '&' 'mut'? Pattern

BoxPattern = 'box' Pattern

ArrayPattern =
    '[' (Pattern (',' Pattern)* (',' RestPattern)? ','?)? ']'

RestPattern = '..' Identifier?
```

### 4.3 Pattern Semantics

#### 4.3.1 Literal Patterns

Match against constant values:

```covibe
match x:
    case 0 => "zero"
    case 1 => "one"
    case 42 => "the answer"
    case _ => "something else"
```

#### 4.3.2 Identifier Patterns

Bind the matched value to a variable:

```covibe
match value:
    case x => print(f"Got: {x}")

# With type constraint
match value:
    case x: int => print(f"Integer: {x}")
```

**Mutable Bindings:**

```covibe
match value:
    case mut x =>
        x += 1
        print(x)
```

**As Patterns:**

Bind a value while also matching a subpattern:

```covibe
match point:
    case p @ Point { x: 0, y: 0 } =>
        print(f"Origin: {p}")
    case Point { x, y } =>
        print(f"Point: ({x}, {y})")
```

#### 4.3.3 Wildcard Pattern

Matches anything without binding:

```covibe
match value:
    case Some(_) => "has a value"
    case None => "no value"
```

#### 4.3.4 Tuple Patterns

Destructure tuples:

```covibe
match pair:
    case (0, 0) => "origin"
    case (0, y) => f"on y-axis at {y}"
    case (x, 0) => f"on x-axis at {x}"
    case (x, y) => f"point ({x}, {y})"
```

#### 4.3.5 Struct Patterns

Destructure struct fields:

```covibe
struct Point:
    x: int
    y: int

match point:
    case Point { x: 0, y: 0 } => "origin"
    case Point { x, y: 0 } => f"on x-axis at {x}"
    case Point { x: 0, y } => f"on y-axis at {y}"
    case Point { x, y } => f"point ({x}, {y})"
```

**Field Shorthand:**

```covibe
match point:
    case Point { x, y } => print(f"{x}, {y}")
    # Equivalent to Point { x: x, y: y }
```

**Partial Matching with Rest:**

```covibe
struct User:
    name: str
    age: int
    email: str
    phone: str

match user:
    case User { name, age, .. } =>
        # Only extract name and age, ignore other fields
        print(f"{name} is {age} years old")
```

#### 4.3.6 Enum Patterns

Match enum variants:

```covibe
enum Option<T>:
    Some(T)
    None

match maybe_value:
    case Some(x) => print(f"Value: {x}")
    case None => print("No value")

enum Message:
    Quit
    Move:
        x: int
        y: int
    Write(str)
    ChangeColor(int, int, int)

match msg:
    case Quit => quit()
    case Move { x, y } => move_to(x, y)
    case Write(text) => write(text)
    case ChangeColor(r, g, b) => set_color(r, g, b)
```

#### 4.3.7 Or Patterns

Match multiple patterns:

```covibe
match character:
    case 'a' | 'e' | 'i' | 'o' | 'u' => "vowel"
    case 'y' => "sometimes vowel"
    case _ => "consonant"

match value:
    case Some(0) | None => "zero or none"
    case Some(x) => f"value: {x}"
```

#### 4.3.8 Range Patterns

Match ranges of values:

```covibe
match age:
    case 0..13 => "child"
    case 13..18 => "teenager"
    case 18..65 => "adult"
    case 65.. => "senior"

match grade:
    case 90..=100 => "A"
    case 80..=89 => "B"
    case 70..=79 => "C"
    case 60..=69 => "D"
    case _ => "F"
```

#### 4.3.9 Reference Patterns

Match references:

```covibe
match &value:
    case &0 => "zero"
    case &x => print(f"reference to {x}")

match &mut_value:
    case &mut x =>
        x += 1  # Can modify through mutable reference
```

#### 4.3.10 Box Patterns

Match boxed values:

```covibe
match boxed_value:
    case box 0 => "boxed zero"
    case box x => print(f"boxed value: {x}")
```

#### 4.3.11 Array/Slice Patterns

Match arrays and slices:

```covibe
match array:
    case [] => "empty"
    case [x] => f"single element: {x}"
    case [first, second] => f"two elements: {first}, {second}"
    case [first, ..rest] => f"first: {first}, rest: {rest}"
    case [.., last] => f"last: {last}"
    case [first, ..middle, last] => f"first: {first}, last: {last}"
```

### 4.4 Guards

Guards add additional conditions to patterns:

```covibe
match point:
    case Point { x, y } if x == y => "on diagonal"
    case Point { x, y } if x > y => "above diagonal"
    case Point { x, y } if x < y => "below diagonal"
    case Point { x, y } => "on diagonal"

match value:
    case x if x > 0 => "positive"
    case x if x < 0 => "negative"
    case _ => "zero"
```

Guards can reference variables bound in the pattern:

```covibe
match pair:
    case (x, y) if x + y == 10 => "sum is 10"
    case (x, y) if x * y > 100 => "product is large"
    case _ => "other"
```

### 4.5 Exhaustiveness Checking

The compiler enforces that all match expressions are exhaustive—every possible value must be handled.

#### 4.5.1 Complete Coverage

```covibe
# Exhaustive - covers all bool values
match flag:
    case true => "yes"
    case false => "no"

# Exhaustive - wildcard catches everything
match value:
    case 0 => "zero"
    case _ => "non-zero"
```

#### 4.5.2 Non-Exhaustive Errors

```covibe
# Compile error: non-exhaustive
match flag:
    case true => "yes"
    # ERROR: pattern `false` not covered

# Compile error: non-exhaustive enum
enum Color:
    Red
    Green
    Blue

match color:
    case Red => "red"
    case Green => "green"
    # ERROR: pattern `Blue` not covered
```

#### 4.5.3 Exhaustiveness with Ranges

```covibe
match value:
    case 0..=100 => "in range"
    case _ => "out of range"  # Required for exhaustiveness
```

#### 4.5.4 Nested Exhaustiveness

```covibe
match (option1, option2):
    case (Some(x), Some(y)) => x + y
    case (Some(x), None) => x
    case (None, Some(y)) => y
    case (None, None) => 0
# All four combinations are covered
```

### 4.6 Usefulness Checking

The compiler warns about unreachable patterns:

```covibe
match x:
    case _ => "anything"
    case 0 => "zero"  # WARNING: unreachable pattern
```

```covibe
match value:
    case x if x > 0 => "positive"
    case y if y > 5 => "greater than 5"  # WARNING: unreachable (subset of previous)
    case _ => "other"
```

---

## 5. Lambda Expressions and Closures

### 5.1 Lambda Syntax

```ebnf
LambdaExpression =
    | 'lambda' Parameters ('->' TypeExpression)? ':' Expression
    | '|' Parameters '|' ('->' TypeExpression)? Expression
    | '|' Parameters '|' ('->' TypeExpression)? Block

Parameters = Parameter (',' Parameter)* ','?

Parameter = Pattern (':' TypeExpression)?
```

### 5.2 Lambda Semantics

#### 5.2.1 Simple Lambdas

```covibe
# Lambda with keyword syntax
let square = lambda x: x * x

# Lambda with closure syntax (preferred)
let square = |x| x * x

# With type annotations
let add: def(int, int) -> int = |a: int, b: int| -> int { a + b }
```

#### 5.2.2 Multi-Parameter Lambdas

```covibe
let add = |a, b| a + b
let multiply = |x, y, z| x * y * z

# With destructuring
let distance = |(x1, y1), (x2, y2)| {
    let dx = x2 - x1
    let dy = y2 - y1
    (dx * dx + dy * dy).sqrt()
}
```

#### 5.2.3 Zero-Parameter Lambdas

```covibe
let get_random = || random()
let greet = || print("Hello!")
```

### 5.3 Closures

Closures capture variables from their enclosing scope.

#### 5.3.1 Capture by Reference

By default, closures capture variables by immutable reference:

```covibe
let x = 10
let f = || print(x)  # Captures &x
f()  # Prints 10
```

#### 5.3.2 Capture by Mutable Reference

Closures that mutate captured variables require `mut`:

```covibe
var counter = 0
let mut increment = || {
    counter += 1
    counter
}
print(increment())  # Prints 1
print(increment())  # Prints 2
```

#### 5.3.3 Capture by Move

Use the `move` keyword to transfer ownership:

```covibe
let data = vec![1, 2, 3]
let f = move || {
    print(data.len())  # Takes ownership of data
}
f()
# data is no longer accessible here
```

#### 5.3.4 Closure Traits

Closures implement one or more of these traits based on their capture behavior:

- **Fn**: Can be called multiple times with immutable access
- **FnMut**: Can be called multiple times with mutable access
- **FnOnce**: Can be called once, consumes captured variables

```covibe
def call_twice<F: Fn()>(f: F):
    f()
    f()

def call_with_mut<F: FnMut()>(mut f: F):
    f()

def call_once<F: FnOnce()>(f: F):
    f()
```

### 5.4 Function Types

```ebnf
FunctionType =
    'def' '(' (TypeExpression (',' TypeExpression)* ','?)? ')'
    ('->' TypeExpression)?
```

Examples:

```covibe
# Function pointer type
let f: def(int, int) -> int = add

# Closure type (generic over closure traits)
def apply<F: Fn(int) -> int>(f: F, x: int) -> int:
    return f(x)

apply(|x| x * 2, 5)  # Returns 10
```

---

## 6. Generator Functions

### 6.1 Generator Syntax

```ebnf
GeneratorFunction =
    Decorator* Visibility? 'def' Identifier
    GenericParams? '(' FunctionParams? ')' ('->' TypeExpression)?
    ':' GeneratorBlock

GeneratorBlock = Block containing at least one 'yield' statement

YieldStatement = 'yield' Expression NEWLINE
```

### 6.2 Generator Semantics

Generators are functions that can pause execution and resume later, producing a sequence of values.

#### 6.2.1 Basic Generator

```covibe
def count_up(n: int):
    var i = 0
    while i < n:
        yield i
        i += 1

for x in count_up(5):
    print(x)  # Prints 0, 1, 2, 3, 4
```

#### 6.2.2 Generator Return Type

Generators return `Generator<T>` where `T` is the type of yielded values:

```covibe
def fibonacci() -> Generator<int>:
    var a = 0
    var b = 1
    loop:
        yield a
        let temp = a
        a = b
        b = temp + b

let fib = fibonacci()
print(fib.next())  # Some(0)
print(fib.next())  # Some(1)
print(fib.next())  # Some(1)
print(fib.next())  # Some(2)
```

#### 6.2.3 Finite Generators

Generators can return to signal completion:

```covibe
def range_gen(start: int, end: int) -> Generator<int>:
    var i = start
    while i < end:
        yield i
        i += 1
    return  # Generator completes

for x in range_gen(0, 5):
    print(x)  # Prints 0, 1, 2, 3, 4
```

#### 6.2.4 Generator with Value Return

Generators can return a final value:

```covibe
def sum_and_count() -> Generator<int, int>:
    var sum = 0
    var count = 0
    loop:
        let value = yield sum
        if value is None:
            return count
        sum += value
        count += 1
```

#### 6.2.5 Generator State

Generators maintain state between yields:

```covibe
def stateful_gen():
    var state = "initial"
    yield state
    state = "modified"
    yield state
    state = "final"
    yield state

let g = stateful_gen()
print(g.next())  # Some("initial")
print(g.next())  # Some("modified")
print(g.next())  # Some("final")
print(g.next())  # None
```

### 6.3 Generator Expressions

Generator expressions provide a concise syntax for simple generators:

```ebnf
GeneratorExpression =
    '(' Expression ComprehensionClause+ ')'

ComprehensionClause =
    | 'for' Pattern 'in' Expression
    | 'if' Expression
```

Examples:

```covibe
# Generator expression (lazy)
let squares = (x * x for x in 0..10)

# With filter
let even_squares = (x * x for x in 0..10 if x % 2 == 0)

# Nested
let pairs = ((x, y) for x in 0..5 for y in 0..5)
```

---

## 7. Comprehensions

### 7.1 List Comprehensions

```ebnf
ListComprehension =
    '[' Expression ComprehensionClause+ ']'
```

Examples:

```covibe
# Basic list comprehension
let squares = [x * x for x in 0..10]

# With filter
let even_squares = [x * x for x in 0..10 if x % 2 == 0]

# Nested comprehension
let matrix = [[0 for _ in 0..cols] for _ in 0..rows]

# Multiple iterators
let pairs = [(x, y) for x in 0..3 for y in 0..3]
# Results in: [(0,0), (0,1), (0,2), (1,0), (1,1), (1,2), (2,0), (2,1), (2,2)]
```

### 7.2 Set Comprehensions

```ebnf
SetComprehension =
    '{' Expression ComprehensionClause+ '}'
```

Examples:

```covibe
# Set comprehension (unique values)
let unique_remainders = {x % 3 for x in 0..10}
# Results in: {0, 1, 2}

# With filter
let vowels = {c for c in "hello world" if c in "aeiou"}
```

### 7.3 Map Comprehensions

```ebnf
MapComprehension =
    '{' Expression ':' Expression ComprehensionClause+ '}'
```

Examples:

```covibe
# Map comprehension
let squares_map = {x: x * x for x in 0..5}
# Results in: {0: 0, 1: 1, 2: 4, 3: 9, 4: 16}

# From pairs
let word_lengths = {word: word.len() for word in words}

# With filter
let positive_only = {k: v for (k, v) in items if v > 0}
```

### 7.4 Comprehension Clause Semantics

#### 7.4.1 Evaluation Order

Comprehension clauses are evaluated left to right:

```covibe
# Equivalent to:
# for x in range1:
#     for y in range2:
#         if condition:
#             yield (x, y)
let result = [(x, y) for x in range1 for y in range2 if condition]
```

#### 7.4.2 Scope

Variables bound in comprehension clauses are scoped to the comprehension:

```covibe
let x = 10
let squares = [x * x for x in 0..5]
print(x)  # Prints 10 (outer x is unchanged)
```

#### 7.4.3 Multiple Filters

Multiple `if` clauses act as logical AND:

```covibe
let result = [x for x in 0..100 if x % 2 == 0 if x % 3 == 0]
# Equivalent to:
# [x for x in 0..100 if (x % 2 == 0) and (x % 3 == 0)]
```

---

## 8. Decorator and Annotation System

### 8.1 Decorator Syntax

```ebnf
Decorator = '@' PostfixExpression NEWLINE

DecoratedDeclaration =
    Decorator+
    (FunctionDeclaration | StructDeclaration | ClassDeclaration)
```

### 8.2 Function Decorators

Decorators are functions that transform other functions:

```covibe
@measure_time
def slow_computation():
    # ... expensive operation
    pass

# Decorator with arguments
@cache(max_size=128)
def fibonacci(n: int) -> int:
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)
```

### 8.3 Built-in Decorators

#### 8.3.1 @inline

Suggests the compiler to inline the function:

```covibe
@inline
def fast_add(a: int, b: int) -> int:
    return a + b
```

#### 8.3.2 @deprecated

Marks a function as deprecated:

```covibe
@deprecated("Use new_function instead")
def old_function():
    pass
```

#### 8.3.3 @test

Marks a function as a test:

```covibe
@test
def test_addition():
    assert add(2, 2) == 4
```

#### 8.3.4 @derive

Automatically implements traits for types:

```covibe
@derive(Debug, Clone, PartialEq)
struct Point:
    x: int
    y: int
```

#### 8.3.5 @comptime

Evaluates function at compile time:

```covibe
@comptime
def factorial(n: int) -> int:
    if n <= 1:
        return 1
    return n * factorial(n - 1)

const FACT_10 = factorial(10)  # Computed at compile time
```

### 8.4 Custom Decorators

Decorators are functions that take a function and return a function:

```covibe
def measure_time<F: Fn(...)>(f: F) -> F:
    def wrapper(...args):
        let start = time.now()
        let result = f(...args)
        let elapsed = time.now() - start
        print(f"Execution time: {elapsed}")
        return result
    return wrapper

# With parameters
def repeat(n: int):
    def decorator<F: Fn(...)>(f: F) -> F:
        def wrapper(...args):
            for _ in 0..n:
                f(...args)
        return wrapper
    return decorator

@repeat(3)
def greet(name: str):
    print(f"Hello, {name}!")

greet("Alice")  # Prints "Hello, Alice!" three times
```

### 8.5 Multiple Decorators

Multiple decorators are applied bottom-up:

```covibe
@decorator_a
@decorator_b
@decorator_c
def function():
    pass

# Equivalent to:
# function = decorator_a(decorator_b(decorator_c(function)))
```

### 8.6 Class/Struct Decorators

Decorators can also transform type declarations:

```covibe
@dataclass
@derive(Debug, Clone)
struct User:
    name: str
    age: int
    email: str

# The @dataclass decorator automatically generates:
# - Constructor
# - Getters/setters
# - __eq__ and __hash__
# - __repr__
```

### 8.7 Decorator Metadata

Decorators can attach metadata to declarations for reflection:

```covibe
@api_endpoint("/users", method="POST")
def create_user(data: UserData):
    # ...

# Metadata can be accessed via reflection
let metadata = reflect(create_user).get_decorator("api_endpoint")
```

---

## Conclusion

This specification defines the complete control flow constructs (if/elif/else, for, while, loop, break, continue, return), function declaration and invocation semantics (including generics, async functions, default arguments, variadic parameters), comprehensive pattern matching with exhaustiveness checking, lambda expressions and closures with capture semantics, generator functions, comprehensions for lists/sets/maps, and a powerful decorator system for code transformation and metadata attachment.

These features combine to provide CoVibe with a rich, expressive syntax for controlling program flow while maintaining the clarity and readability that are core design goals of the language.

---

**Document History:**
- 2026-04-14: Initial version 1.0 — Complete control flow, functions, and pattern matching grammar
