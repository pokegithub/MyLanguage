# CoVibe Language Specification
## Part 4: Memory Model and Ownership System

**Version:** 1.0
**Date:** 2026-04-14
**Status:** Final

---

## Table of Contents

1. [Introduction](#introduction)
2. [Memory Layout and Representation](#memory-layout-and-representation)
3. [Ownership Rules](#ownership-rules)
4. [Move Semantics](#move-semantics)
5. [Copy Semantics](#copy-semantics)
6. [Borrow Rules](#borrow-rules)
7. [Lifetime System](#lifetime-system)
8. [Reborrowing](#reborrowing)
9. [Borrow Splitting](#borrow-splitting)
10. [Compile-Time Reference Counting](#compile-time-reference-counting)
11. [Stack vs Heap Allocation](#stack-vs-heap-allocation)
12. [Smart Pointers](#smart-pointers)
13. [Slices and Bounds Checking](#slices-and-bounds-checking)
14. [Custom Allocator Interface](#custom-allocator-interface)
15. [RAII and Drop Order](#raii-and-drop-order)
16. [Defer Statement](#defer-statement)
17. [Pin and Unpin](#pin-and-unpin)

---

## 1. Introduction

This document specifies the memory model and ownership system of the CoVibe programming language. CoVibe's memory management strategy is designed to provide:

- **Memory safety without garbage collection**: All memory safety violations are caught at compile time
- **Predictable performance**: No GC pauses, deterministic allocation and deallocation
- **Zero-cost abstractions**: The ownership system compiles to the same code as hand-optimized C
- **Flexible allocation strategies**: Support for stack, heap, arena, and custom allocators

The ownership system is inspired by Rust's borrow checker but with simplified syntax and optional compile-time reference counting for ease-of-use scenarios.

### 1.1 Design Principles

1. **Safety by Default**: Memory errors are impossible in safe code
2. **Explicit Unsafety**: Unsafe operations require explicit `unsafe` blocks
3. **Zero Runtime Cost**: All ownership checks happen at compile time
4. **Ergonomic**: Common patterns should be concise and readable
5. **Predictable**: Memory behavior is deterministic and documented

### 1.2 Memory Safety Guarantees

CoVibe guarantees the following at compile time:

- **No null pointer dereferences**: Null values are explicit via `Option<T>`
- **No dangling pointers**: References cannot outlive their referents
- **No use-after-free**: Values cannot be used after being moved or dropped
- **No double-free**: Values are dropped exactly once
- **No data races**: Shared mutable state is prohibited (enforced by ownership + concurrency rules)
- **No buffer overflows**: Array indexing is bounds-checked (with opt-out for performance)

---

## 2. Memory Layout and Representation

### 2.1 Value Representation

Every type in CoVibe has a well-defined layout in memory:

**Primitive Types:**
```
i8, u8:     1 byte, alignment 1
i16, u16:   2 bytes, alignment 2
i32, u32:   4 bytes, alignment 4
i64, u64:   8 bytes, alignment 8
i128, u128: 16 bytes, alignment 16
isize, usize: pointer-sized (4 or 8 bytes)
f32:        4 bytes, alignment 4
f64:        8 bytes, alignment 8
bool:       1 byte, alignment 1
char:       4 bytes (UTF-32), alignment 4
```

**Pointer Types:**
```
&T:         pointer-sized
&mut T:     pointer-sized
*const T:   pointer-sized
*mut T:     pointer-sized
Box<T>:     pointer-sized (thin pointer)
```

**Compound Types:**
```
(T1, T2, ...):  Layout with proper alignment padding
[T; N]:         N * sizeof(T) with alignment of T
struct:         Layout follows C ABI with padding
enum:           Discriminant + max(variant sizes)
```

### 2.2 Struct Layout

Structs follow C ABI layout rules by default:

```covibe
struct Point:
    x: f64  # Offset 0, size 8
    y: f64  # Offset 8, size 8
# Total size: 16 bytes, alignment: 8

struct Mixed:
    a: u8   # Offset 0, size 1
    # Padding: 3 bytes
    b: u32  # Offset 4, size 4
    c: u16  # Offset 8, size 2
    # Padding: 6 bytes (for alignment)
# Total size: 16 bytes, alignment: 4
```

**Explicit Layout Control:**
```covibe
@repr(C)
struct CCompatible:
    # Guaranteed C ABI layout
    field1: int
    field2: float

@repr(packed)
struct NoPadding:
    # No padding, tightly packed
    a: u8
    b: u32  # Offset 1, not 4!

@repr(align(16))
struct Aligned:
    # Force 16-byte alignment
    data: [f32; 4]
```

### 2.3 Enum Layout

Enums are represented with a discriminant tag:

```covibe
enum Option<T>:
    Some(T)
    None

# Layout:
# - Discriminant: typically 1 byte (optimized based on variant count)
# - Payload: max(sizeof(T), 0)
# - Alignment: max(align(discriminant), align(T))
```

**Enum Optimization:**

CoVibe performs several enum layout optimizations:

1. **Null Pointer Optimization**: `Option<&T>` is the same size as `&T`
   ```covibe
   # sizeof(Option<&i32>) == sizeof(&i32) == pointer size
   ```

2. **Niche Filling**: Use invalid values as discriminants
   ```covibe
   # sizeof(Option<bool>) == 1 byte
   # None = 2, Some(false) = 0, Some(true) = 1
   ```

3. **Discriminant Elision**: Single-variant enums have no discriminant
   ```covibe
   enum Wrapper<T>:
       Value(T)
   # sizeof(Wrapper<T>) == sizeof(T)
   ```

### 2.4 Fat Pointers

Slices and trait objects are represented as fat pointers (two words):

```covibe
&[T]:        (data_ptr: *const T, len: usize)
&mut [T]:    (data_ptr: *mut T, len: usize)
&str:        (data_ptr: *const u8, len: usize)
&dyn Trait:  (data_ptr: *const (), vtable: *const VTable)
```

### 2.5 Zero-Sized Types

Types with no data have size and alignment of 0:

```covibe
struct Unit:
    pass

# sizeof(Unit) == 0
# Optimized away in memory but retain semantics

def use_unit(u: Unit):
    # u takes no space but function still type-checks
    pass
```

---

## 3. Ownership Rules

### 3.1 The Three Rules of Ownership

1. **Each value has a single owner**
   - A value is owned by exactly one variable at any time
   - The owner is responsible for deallocating the value

2. **Ownership can be transferred (moved)**
   - When ownership is transferred, the previous owner can no longer access the value
   - Moves happen on assignment, function calls, and returns

3. **When the owner goes out of scope, the value is dropped**
   - The value's destructor (`drop`) is called automatically
   - Memory is freed

### 3.2 Ownership Transfer

```covibe
let x = Box::new(42)  # x owns the Box
let y = x             # Ownership transferred to y
# x is no longer valid
print(y)              # OK
# print(x)            # ERROR: use of moved value
```

**Formal Rule:**
```
Γ ⊢ x : T    T is not Copy
────────────────────────────
Γ \ {x} ⊢ y = x : T
# After assignment, x is removed from environment
```

### 3.3 Scope-Based Deallocation

```covibe
def example():
    let x = String::from("hello")  # x owns the string
    let y = String::from("world")  # y owns the string
    # At end of scope:
    # 1. y is dropped
    # 2. x is dropped (reverse order of declaration)

# Memory is freed automatically, no manual cleanup needed
```

### 3.4 Ownership in Function Calls

**Passing by Value (Move):**
```covibe
def take_ownership(s: String):
    print(s)
    # s is dropped here

let my_string = String::from("hello")
take_ownership(my_string)  # my_string moved into function
# my_string is no longer valid here
```

**Returning Ownership:**
```covibe
def create_string() -> String:
    let s = String::from("hello")
    return s  # Ownership transferred to caller

let result = create_string()  # result owns the string
```

### 3.5 Ownership and Pattern Matching

```covibe
let pair = (String::from("hello"), String::from("world"))

match pair:
    case (s1, s2) =>
        # s1 and s2 own the strings, pair is consumed
        print(s1)
        print(s2)

# pair is no longer valid
```

**Partial Move:**
```covibe
struct TwoStrings:
    first: String
    second: String

let ts = TwoStrings {
    first: String::from("a"),
    second: String::from("b")
}

let f = ts.first  # Move out first field
# ts is now partially moved
# ts.first is invalid
# ts.second is still valid
# ts as a whole is invalid
```

---

## 4. Move Semantics

### 4.1 Move vs Copy

By default, all types have **move semantics**:

```covibe
struct Resource:
    data: Vec<int>

let r1 = Resource { data: vec![1, 2, 3] }
let r2 = r1  # MOVE: r1 is no longer valid
```

Only types that implement the `Copy` trait have copy semantics.

### 4.2 Explicit Move

Use the `move` keyword to make moves explicit:

```covibe
let x = String::from("hello")
let y = move x  # Explicit move, x is no longer valid
```

This is particularly useful in closures:

```covibe
let data = vec![1, 2, 3]
let closure = move || {
    # closure takes ownership of data
    print(data)
}
closure()
# data is no longer accessible here
```

### 4.3 Move Semantics in Control Flow

**If Expressions:**
```covibe
let x = String::from("hello")
let y = if condition:
    x  # x moved here
else:
    String::from("world")

# x is invalid regardless of which branch was taken
```

**Match Expressions:**
```covibe
let opt = Some(String::from("hello"))
match opt:
    case Some(s) =>
        # s owns the string
        print(s)
    case None =>
        pass

# opt is consumed, no longer valid
```

### 4.4 Move Paths

CoVibe tracks moves at the field level:

```covibe
struct Pair:
    a: String
    b: String

let p = Pair {
    a: String::from("first"),
    b: String::from("second")
}

let x = p.a  # Move p.a

# p is now partially moved:
# - p as a whole: INVALID
# - p.a: INVALID (moved)
# - p.b: VALID (not moved)

print(p.b)   # OK
# print(p.a) # ERROR: use of moved value
# print(p)   # ERROR: use of partially moved value
```

### 4.5 Move Errors

The compiler prevents use of moved values:

```covibe
let s = String::from("hello")
let t = s  # Move
print(s)   # ERROR: value used after move

# Error message:
# error[E0382]: use of moved value: `s`
#  --> example.cv:3:7
#   |
# 2 | let t = s  # Move
#   |         - value moved here
# 3 | print(s)   # ERROR: value used after move
#   |       ^ value used here after move
#   |
#   = note: move occurs because `s` has type `String`, which does not implement the `Copy` trait
```

---

## 5. Copy Semantics

### 5.1 The Copy Trait

Types that implement `Copy` are bitwise copyable:

```covibe
trait Copy:
    # Marker trait - no methods
    pass
```

**Requirements for Copy:**
1. Type must be `Clone`
2. Type contains only `Copy` fields
3. Type has no custom `drop` implementation

### 5.2 Built-in Copy Types

The following types are `Copy` by default:

- All integer types: `i8`, `i16`, `i32`, `i64`, `i128`, `isize`, `u8`, `u16`, `u32`, `u64`, `u128`, `usize`
- All floating-point types: `f32`, `f64`
- `bool`
- `char`
- Function pointers: `def(T) -> U`
- Raw pointers: `*const T`, `*mut T`
- Tuples of `Copy` types: `(T, U)` where `T: Copy` and `U: Copy`
- Arrays of `Copy` types: `[T; N]` where `T: Copy`
- Shared references: `&T` (but NOT `&mut T`)

### 5.3 Deriving Copy

Structs and enums can derive `Copy`:

```covibe
@derive(Copy, Clone)
struct Point:
    x: int
    y: int

let p1 = Point { x: 5, y: 10 }
let p2 = p1  # COPY: p1 is still valid
print(p1.x)  # OK: p1 was copied, not moved
```

**Cannot derive Copy if:**
```covibe
struct Invalid:
    data: String  # String is not Copy

# ERROR: Cannot derive Copy because String is not Copy
# @derive(Copy)  # This would fail
struct Invalid
```

### 5.4 Copy vs Clone

**Copy**: Implicit, bitwise copy
```covibe
let x = 42
let y = x  # Implicit copy
```

**Clone**: Explicit, potentially expensive
```covibe
let s1 = String::from("hello")
let s2 = s1.clone()  # Explicit deep copy
# Both s1 and s2 are valid
```

### 5.5 Copy Semantics in Functions

```covibe
def use_value(x: int):
    print(x)

let num = 42
use_value(num)  # num is copied
print(num)      # OK: num is still valid (int is Copy)

def use_string(s: String):
    print(s)

let text = String::from("hello")
use_string(text)  # text is moved
# print(text)     # ERROR: text was moved
```

---

## 6. Borrow Rules

### 6.1 References

CoVibe has two types of references:

**Shared Reference (`&T`):**
- Read-only access
- Multiple shared references can coexist
- Immutable: cannot modify the referenced value

**Mutable Reference (`&mut T`):**
- Read-write access
- Exclusive: only one mutable reference allowed at a time
- No other references (shared or mutable) can coexist

### 6.2 The Borrowing Rules

1. **At any given time, you can have EITHER:**
   - One mutable reference (`&mut T`), OR
   - Any number of shared references (`&T`)

2. **References must always be valid:**
   - References cannot outlive the data they refer to
   - No dangling references

### 6.3 Shared References

```covibe
let x = 5
let r1 = &x  # Shared reference
let r2 = &x  # Another shared reference - OK!
let r3 = &x  # Yet another - OK!

print(*r1)   # OK: 5
print(*r2)   # OK: 5
print(*r3)   # OK: 5
```

**Cannot Modify Through Shared Reference:**
```covibe
let x = 5
let r = &x
# *r = 10  # ERROR: cannot assign to `*r` which is behind a `&` reference
```

### 6.4 Mutable References

```covibe
var x = 5
let r = &mut x  # Mutable reference

*r = 10        # OK: can modify through &mut
print(*r)      # OK: 10
```

**Exclusivity Rule:**
```covibe
var x = 5
let r1 = &mut x
# let r2 = &mut x  # ERROR: cannot borrow `x` as mutable more than once

# Cannot mix shared and mutable references:
var y = 5
let r1 = &y
# let r2 = &mut y  # ERROR: cannot borrow `y` as mutable because it is also borrowed as immutable
```

### 6.5 Borrow Checking Example

```covibe
def main():
    var s = String::from("hello")

    let r1 = &s      # Shared borrow
    let r2 = &s      # Another shared borrow - OK
    print(r1)
    print(r2)
    # r1 and r2 go out of scope here

    let r3 = &mut s  # Mutable borrow - OK (no other active borrows)
    r3.push_str(" world")
    print(r3)
```

**Borrow Checker Error:**
```covibe
var s = String::from("hello")
let r1 = &s          # Shared borrow
let r2 = &mut s      # ERROR: cannot borrow as mutable

# Error message:
# error[E0502]: cannot borrow `s` as mutable because it is also borrowed as immutable
#  --> example.cv:3:10
#   |
# 2 | let r1 = &s          # Shared borrow
#   |          -- immutable borrow occurs here
# 3 | let r2 = &mut s      # ERROR
#   |          ^^^^^^ mutable borrow occurs here
# 4 | print(r1)
#   |       -- immutable borrow later used here
```

### 6.6 Borrow Scopes (Non-Lexical Lifetimes)

CoVibe uses **Non-Lexical Lifetimes (NLL)**: borrows end when they're last used, not at scope end:

```covibe
var s = String::from("hello")

let r1 = &s
let r2 = &s
print(r1)
print(r2)
# r1 and r2 are no longer used after this point

let r3 = &mut s  # OK: shared borrows have ended
r3.push_str(" world")
```

Without NLL, this would be an error because `r1` and `r2` would be considered active until the end of the scope.

### 6.7 Method Call Borrowing

Method calls automatically borrow `self`:

```covibe
struct Counter:
    count: int

impl Counter:
    def increment(&mut self):
        self.count += 1

    def get(&self) -> int:
        return self.count

var c = Counter { count: 0 }
c.increment()  # Borrows &mut c
let x = c.get()  # Borrows &c
```

---

## 7. Lifetime System

### 7.1 Lifetime Annotations

Lifetimes track how long references are valid:

```covibe
def longest<'a>(s1: &'a str, s2: &'a str) -> &'a str:
    if s1.len() > s2.len():
        return s1
    else:
        return s2
```

**Lifetime Syntax:**
```ebnf
Lifetime = "'" Identifier

LifetimeParam = Lifetime (':' LifetimeBound)?

LifetimeBound = Lifetime ('+' Lifetime)*
```

### 7.2 Lifetime Elision Rules

In many cases, lifetimes can be elided (inferred):

**Rule 1**: Each elided reference parameter gets its own lifetime:
```covibe
# Written:
def foo(x: &str, y: &str) -> &str

# Desugars to:
def foo<'a, 'b>(x: &'a str, y: &'b str) -> &str
```

**Rule 2**: If there's exactly one input lifetime, it's assigned to all output lifetimes:
```covibe
# Written:
def first(s: &str) -> &str

# Desugars to:
def first<'a>(s: &'a str) -> &'a str
```

**Rule 3**: If there's a `&self` or `&mut self` parameter, its lifetime is assigned to all output lifetimes:
```covibe
# Written:
impl String:
    def as_str(&self) -> &str

# Desugars to:
impl String:
    def as_str<'a>(&'a self) -> &'a str
```

### 7.3 Lifetime Bounds

Lifetimes can constrain each other:

```covibe
# 'b must outlive 'a
def example<'a, 'b: 'a>(x: &'a str, y: &'b str) -> &'a str:
    return x  # OK: 'a outlives itself
    # return y  # OK: 'b outlives 'a
```

**Subtyping:**
```
'static:  The lifetime of the entire program
'a: 'b    Read as: "'a outlives 'b"

If 'a: 'b, then &'a T <: &'b T
```

### 7.4 The 'static Lifetime

`'static` is the lifetime of the entire program:

```covibe
let s: &'static str = "Hello"  # String literal lives forever

const GREETING: &'static str = "Hello, world!"

static CONFIG: Config = Config { /* ... */ }
```

### 7.5 Lifetime in Structs

Structs containing references must declare lifetimes:

```covibe
struct Excerpt<'a>:
    text: &'a str

impl<'a> Excerpt<'a>:
    def new(text: &'a str) -> Excerpt<'a>:
        return Excerpt { text }

    def announce(&self):
        print(f"Excerpt: {self.text}")
```

**Multiple Lifetimes:**
```covibe
struct TwoRefs<'a, 'b>:
    first: &'a str
    second: &'b str

# Using the struct:
let s1 = String::from("hello")
let s2 = String::from("world")
let refs = TwoRefs {
    first: &s1,
    second: &s2
}
```

### 7.6 Lifetime Inference

The compiler infers lifetimes in most cases:

```covibe
struct Context:
    data: String

impl Context:
    # Compiler infers: def get_data<'a>(&'a self) -> &'a str
    def get_data(&self) -> &str:
        return &self.data
```

### 7.7 Higher-Rank Trait Bounds (HRTB)

For functions that work with any lifetime:

```covibe
trait DoSomething:
    def process(&self, s: &str)

# Accepts any implementation of DoSomething
def apply<T>(handler: T)
where
    T: for<'a> DoSomething<'a>:
    handler.process("test")
```

---

## 8. Reborrowing

### 8.1 Reborrow Semantics

A borrow of a borrow creates a new, shorter-lived borrow:

```covibe
def use_ref(r: &String):
    print(r)

let s = String::from("hello")
let r1 = &s
use_ref(r1)   # Reborrow: &s → &&s, then dereferenced to &s
print(r1)     # OK: r1 is still valid
```

### 8.2 Mutable Reborrowing

Mutable references can be reborrowed:

```covibe
def append_world(s: &mut String):
    s.push_str(" world")

var text = String::from("hello")
let r = &mut text
append_world(r)  # Reborrow: &mut &mut String → &mut String
r.push_str("!")  # OK: r is still valid
```

**Reborrow Rules:**
```
&mut T → &T       (immutable reborrow from mutable reference)
&mut T → &mut T   (mutable reborrow)
&T → &T           (shared reborrow)
```

### 8.3 Reborrow Lifetime

Reborrows have a shorter lifetime:

```covibe
var s = String::from("hello")
let r1 = &mut s      # Lifetime 'a
{
    let r2 = &mut *r1  # Reborrow with lifetime 'b, where 'b < 'a
    r2.push_str(" there")
}  # r2 ends here
r1.push_str(" world")  # OK: r1 is still valid
```

### 8.4 Automatic Reborrowing

The compiler automatically reborrows in method calls:

```covibe
impl String:
    def push_str(&mut self, s: &str):
        # ...

var s = String::from("hello")
let r = &mut s

r.push_str(" world")  # Automatically reborrows r
r.push_str("!")       # r is still valid, reborrows again
```

---

## 9. Borrow Splitting

### 9.1 Splitting Struct Fields

Different fields of a struct can be borrowed separately:

```covibe
struct Point:
    x: int
    y: int

var p = Point { x: 0, y: 0 }
let rx = &mut p.x  # Borrow x mutably
let ry = &mut p.y  # Borrow y mutably - OK! Different fields
*rx = 10
*ry = 20
```

**Why This Works:**
The borrow checker understands that `p.x` and `p.y` are disjoint, so mutably borrowing both is safe.

### 9.2 Splitting Array Elements

Array elements can be split with slices:

```covibe
var arr = [1, 2, 3, 4, 5]
let (left, right) = arr.split_at_mut(2)
# left = &mut [1, 2]
# right = &mut [3, 4, 5]

left[0] = 10
right[0] = 30
# arr is now [10, 2, 30, 4, 5]
```

### 9.3 Manual Borrow Splitting

Functions can split borrows manually using unsafe code:

```covibe
def split_at_mut<T>(slice: &mut [T], index: usize) -> (&mut [T], &mut [T]):
    unsafe:
        let ptr = slice.as_mut_ptr()
        let len = slice.len()
        assert(index <= len)
        return (
            &mut *ptr.offset(0..index),
            &mut *ptr.offset(index..len)
        )
```

### 9.4 Borrow Splitting Limitations

**Cannot split the same field:**
```covibe
var x = 10
let r1 = &mut x
# let r2 = &mut x  # ERROR: cannot borrow `x` as mutable more than once
```

**Cannot split overlapping slices:**
```covibe
var arr = [1, 2, 3, 4]
let s = &mut arr[..]
let s1 = &mut s[0..2]
# let s2 = &mut s[1..3]  # ERROR: overlaps with s1
```

---

## 10. Compile-Time Reference Counting

### 10.1 Rc<T> - Reference Counted Pointer

`Rc<T>` provides shared ownership with compile-time reference counting:

```covibe
use std.rc.Rc

let a = Rc::new(42)
let b = a.clone()  # Increment reference count
let c = a.clone()  # Increment reference count

print(Rc::strong_count(&a))  # 3

# When a, b, c go out of scope, reference count decreases
# When count reaches 0, value is dropped
```

**Type Signature:**
```covibe
struct Rc<T>:
    # Not Copy, only Clone
    # Implements Deref<Target=T>
    pass
```

### 10.2 Rc<T> Characteristics

**Not Thread-Safe:**
```covibe
# Rc<T> is NOT Send or Sync
# Cannot be shared across threads

# This will fail to compile:
# spawn(move || {
#     let r = Rc::new(42)
# })  # ERROR: Rc<i32> cannot be sent between threads
```

**Immutable Only:**
```covibe
let rc = Rc::new(42)
# *rc = 10  # ERROR: cannot assign to immutable borrowed content
```

### 10.3 Interior Mutability with RefCell<T>

Combine `Rc` with `RefCell` for shared mutable ownership:

```covibe
use std.rc.Rc
use std.cell.RefCell

struct Node:
    value: int
    children: Vec<Rc<RefCell<Node>>>

let node = Rc::new(RefCell::new(Node {
    value: 42,
    children: vec![]
}))

# Mutate through RefCell
node.borrow_mut().value = 100

# Multiple owners
let child = Rc::clone(&node)
```

**RefCell Runtime Checks:**
```covibe
let cell = RefCell::new(42)

let r1 = cell.borrow()      # Immutable borrow - OK
let r2 = cell.borrow()      # Another immutable borrow - OK
# let r3 = cell.borrow_mut() # Runtime panic! Already borrowed immutably

drop(r1)
drop(r2)

let r4 = cell.borrow_mut()  # OK now
*r4 = 100
```

### 10.4 Weak<T> - Weak References

Break reference cycles with weak references:

```covibe
use std.rc::{Rc, Weak}

struct Node:
    value: int
    parent: Option<Weak<Node>>
    children: Vec<Rc<Node>>

# Create a tree without cycles
let parent = Rc::new(Node {
    value: 1,
    parent: None,
    children: vec![]
})

let child = Rc::new(Node {
    value: 2,
    parent: Some(Rc::downgrade(&parent)),  # Weak reference
    children: vec![]
})
```

**Weak Reference API:**
```covibe
let strong = Rc::new(42)
let weak = Rc::downgrade(&strong)

# Upgrade weak to strong (returns Option<Rc<T>>)
match weak.upgrade():
    case Some(rc) => print(*rc)
    case None => print("Value dropped")

drop(strong)

# Now upgrade fails
assert(weak.upgrade().is_none())
```

---

## 11. Stack vs Heap Allocation

### 11.1 Stack Allocation

By default, values are stack-allocated:

```covibe
def example():
    let x = 42           # Stack: primitive type
    let arr = [1, 2, 3]  # Stack: fixed-size array
    let point = Point { x: 0, y: 0 }  # Stack: struct

# All values dropped and stack freed when function returns
```

**Stack Characteristics:**
- Fast allocation (just moving stack pointer)
- Fast deallocation (just moving stack pointer back)
- Limited size (typically a few MB)
- Automatic cleanup (RAII)

### 11.2 Heap Allocation

Use `Box<T>` for explicit heap allocation:

```covibe
let x = Box::new(42)        # Heap: boxed integer
let v = Vec::new()          # Heap: growable array
let s = String::from("hi")  # Heap: growable string
```

**Heap Characteristics:**
- Slower allocation (system allocator call)
- Unlimited size (up to available memory)
- Manual placement (you decide when to allocate)
- Automatic cleanup (still RAII)

### 11.3 The `alloc` Keyword

Explicitly allocate on heap:

```covibe
# Allocate single value
let x = alloc 42  # Equivalent to Box::new(42)

# Allocate array
let arr = alloc [1, 2, 3, 4, 5]

# Allocate struct
let point = alloc Point { x: 10, y: 20 }
```

**Type Transformation:**
```
alloc T → Box<T>
```

### 11.4 Large Types and Stack Overflow

**Problem:**
```covibe
struct Large:
    data: [u8; 1_000_000]  # 1 MB

def example():
    let x = Large { data: [0; 1_000_000] }  # Stack overflow!
```

**Solution:**
```covibe
def example():
    let x = Box::new(Large { data: [0; 1_000_000] })  # OK: on heap
```

### 11.5 Stack vs Heap Guidelines

**Use Stack When:**
- Type has known, small, fixed size
- Short lifetime
- Performance-critical hot path

**Use Heap When:**
- Type has dynamic size (Vec, String, HashMap)
- Large type (> 1 KB as a rule of thumb)
- Need to transfer ownership cheaply (move a pointer, not the data)
- Need recursive types

### 11.6 Escape Analysis Optimization

The compiler can stack-allocate Box values that don't escape:

```covibe
def local_only():
    let x = Box::new(42)  # May be stack-allocated!
    return *x             # Value doesn't escape

# Compiler optimizes to:
def local_only():
    let x = 42
    return x
```

---

## 12. Smart Pointers

### 12.1 Box<T> - Owned Heap Pointer

`Box<T>` is the fundamental owned heap pointer:

```covibe
struct Box<T>:
    # Thin pointer to heap-allocated T
    pass

impl<T> Box<T>:
    def new(value: T) -> Box<T>:
        # Allocate on heap and return pointer
        pass

    def into_inner(self) -> T:
        # Move value out of box, deallocate box
        pass
```

**Characteristics:**
- Single owner
- Deallocates when dropped
- Implements `Deref<Target=T>` and `DerefMut`
- Pointer-sized (thin pointer)

**Usage:**
```covibe
let b = Box::new(42)
let x = *b        # Dereference
let y = b.clone() # ERROR: Box<i32> is not Clone
let z = *b        # Move out of box
# b is now invalid
```

### 12.2 Rc<T> - Shared Ownership (Review)

See section 10.1-10.4 for details.

**Quick Summary:**
```covibe
let rc1 = Rc::new(42)
let rc2 = rc1.clone()  # Increment count
let rc3 = rc1.clone()  # Increment count
# Count: 3
# Dropped when count reaches 0
```

### 12.3 Arc<T> - Atomic Reference Counted

`Arc<T>` is the thread-safe version of `Rc<T>`:

```covibe
use std.sync.Arc

let arc = Arc::new(42)
let arc2 = arc.clone()

spawn(move || {
    print(*arc2)  # OK: Arc is Send + Sync
})

print(*arc)
```

**Characteristics:**
- Thread-safe (atomic reference counting)
- Implements `Send + Sync` (can be shared across threads)
- Slightly slower than `Rc` (atomic operations)

### 12.4 RefCell<T> - Interior Mutability

Runtime-checked mutable borrows:

```covibe
use std.cell.RefCell

let cell = RefCell::new(42)

# Multiple immutable borrows
let r1 = cell.borrow()
let r2 = cell.borrow()
print(*r1 + *r2)  # 84

drop(r1)
drop(r2)

# Single mutable borrow
let mut r3 = cell.borrow_mut()
*r3 = 100
```

**Runtime Panics:**
```covibe
let cell = RefCell::new(42)
let r1 = cell.borrow()
let r2 = cell.borrow_mut()  # PANIC: already borrowed
```

### 12.5 Mutex<T> and RwLock<T>

Thread-safe interior mutability:

```covibe
use std.sync.Mutex

let m = Mutex::new(42)

{
    let mut guard = m.lock().unwrap()
    *guard = 100
}  # Lock released

let value = *m.lock().unwrap()
print(value)  # 100
```

**RwLock:**
```covibe
use std.sync.RwLock

let lock = RwLock::new(42)

# Multiple readers
let r1 = lock.read().unwrap()
let r2 = lock.read().unwrap()
print(*r1 + *r2)

drop(r1)
drop(r2)

# Single writer
let mut w = lock.write().unwrap()
*w = 100
```

### 12.6 Cell<T> - Copy Interior Mutability

For `Copy` types, simpler than `RefCell`:

```covibe
use std.cell.Cell

let cell = Cell::new(42)

cell.set(100)
let x = cell.get()  # 100

# No borrowing, just copying values
```

---

## 13. Slices and Bounds Checking

### 13.1 Slice Types

Slices are views into contiguous sequences:

```covibe
&[T]      # Immutable slice
&mut [T]  # Mutable slice
&str      # String slice (immutable)
```

**Fat Pointer Representation:**
```
&[T] = (ptr: *const T, len: usize)
&str = (ptr: *const u8, len: usize)
```

### 13.2 Creating Slices

**From Arrays:**
```covibe
let arr = [1, 2, 3, 4, 5]
let slice: &[int] = &arr        # Whole array
let s1: &[int] = &arr[1..4]     # Elements 1, 2, 3
let s2: &[int] = &arr[..3]      # Elements 0, 1, 2
let s3: &[int] = &arr[2..]      # Elements 2, 3, 4
let s4: &[int] = &arr[..]       # Whole array
```

**From Vectors:**
```covibe
let v = vec![1, 2, 3, 4, 5]
let slice: &[int] = &v[1..4]
```

**String Slices:**
```covibe
let s = String::from("hello world")
let hello: &str = &s[0..5]
let world: &str = &s[6..11]
```

### 13.3 Bounds Checking

**Compile-Time Bounds Checks:**
```covibe
let arr = [1, 2, 3, 4, 5]
let x = arr[0]    # OK: in bounds
let y = arr[4]    # OK: in bounds
# let z = arr[5]  # ERROR: index out of bounds (if const)
```

**Runtime Bounds Checks:**
```covibe
def get_element(arr: &[int], index: usize) -> int:
    return arr[index]  # Runtime check

let arr = [1, 2, 3, 4, 5]
let x = get_element(&arr, 2)   # OK: 3
# let y = get_element(&arr, 10) # PANIC: index out of bounds
```

### 13.4 Unchecked Indexing

For performance-critical code, opt out of bounds checks:

```covibe
def sum_unchecked(arr: &[int]) -> int:
    var total = 0
    unsafe:
        for i in 0..arr.len():
            # No bounds check
            total += arr.get_unchecked(i)
    return total
```

**Safety Contract:**
The caller must ensure indices are in bounds.

### 13.5 Slice Methods

```covibe
trait Slice<T>:
    def len(&self) -> usize
    def is_empty(&self) -> bool
    def first(&self) -> Option<&T>
    def last(&self) -> Option<&T>
    def get(&self, index: usize) -> Option<&T>
    def get_unchecked(&self, index: usize) -> &T  # unsafe
    def split_at(&self, index: usize) -> (&[T], &[T])
    def windows(&self, size: usize) -> Windows<T>
    def chunks(&self, size: usize) -> Chunks<T>
```

**Usage:**
```covibe
let slice = &[1, 2, 3, 4, 5]

assert(slice.len() == 5)
assert(slice.first() == Some(&1))
assert(slice.last() == Some(&5))
assert(slice.get(2) == Some(&3))
assert(slice.get(10) == None)

let (left, right) = slice.split_at(2)
# left = &[1, 2]
# right = &[3, 4, 5]
```

### 13.6 Mutable Slices

```covibe
var arr = [1, 2, 3, 4, 5]
let slice: &mut [int] = &mut arr[..]

slice[0] = 10
slice[2] = 30

# arr is now [10, 2, 30, 4, 5]
```

**Mutable Slice Methods:**
```covibe
trait SliceMut<T>:
    def swap(&mut self, i: usize, j: usize)
    def reverse(&mut self)
    def sort(&mut self) where T: Ord
    def fill(&mut self, value: T) where T: Clone
    def split_at_mut(&mut self, index: usize) -> (&mut [T], &mut [T])
```

---

## 14. Custom Allocator Interface

### 14.1 The Allocator Trait

```covibe
trait Allocator:
    def allocate(&self, layout: Layout) -> Result<*mut u8, AllocError>
    def deallocate(&self, ptr: *mut u8, layout: Layout)
    def reallocate(
        &self,
        ptr: *mut u8,
        old_layout: Layout,
        new_size: usize
    ) -> Result<*mut u8, AllocError>
    def allocate_zeroed(&self, layout: Layout) -> Result<*mut u8, AllocError>:
        let ptr = self.allocate(layout)?
        unsafe:
            ptr.write_bytes(0, layout.size())
        return Ok(ptr)
```

### 14.2 Layout Description

```covibe
struct Layout:
    size: usize
    align: usize

impl Layout:
    def new<T>() -> Layout:
        return Layout {
            size: sizeof::<T>(),
            align: alignof::<T>()
        }

    def for_value<T>(value: &T) -> Layout:
        return Layout::new::<T>()

    def array<T>(n: usize) -> Result<Layout, LayoutError>:
        let size = sizeof::<T>() * n
        return Ok(Layout {
            size,
            align: alignof::<T>()
        })
```

### 14.3 Built-in Allocators

**Global Allocator:**
```covibe
struct Global:
    pass

impl Allocator for Global:
    def allocate(&self, layout: Layout) -> Result<*mut u8, AllocError>:
        # System allocator (malloc)
        unsafe:
            let ptr = libc::malloc(layout.size)
            if ptr.is_null():
                return Err(AllocError)
            return Ok(ptr as *mut u8)

    def deallocate(&self, ptr: *mut u8, layout: Layout):
        unsafe:
            libc::free(ptr as *mut ())
```

### 14.4 Arena Allocator

Fast bump-pointer allocation:

```covibe
struct Arena:
    buffer: Vec<u8>
    offset: usize

impl Arena:
    def new(capacity: usize) -> Arena:
        return Arena {
            buffer: Vec::with_capacity(capacity),
            offset: 0
        }

impl Allocator for Arena:
    def allocate(&self, layout: Layout) -> Result<*mut u8, AllocError>:
        # Align offset
        let aligned = (self.offset + layout.align - 1) & !(layout.align - 1)
        let new_offset = aligned + layout.size

        if new_offset > self.buffer.capacity():
            return Err(AllocError)

        self.offset = new_offset
        unsafe:
            return Ok(self.buffer.as_mut_ptr().add(aligned))

    def deallocate(&self, ptr: *mut u8, layout: Layout):
        # No-op: arena frees all at once
        pass
```

### 14.5 Pool Allocator

Fixed-size block allocation:

```covibe
struct Pool<T>:
    blocks: Vec<Option<T>>
    free_list: Vec<usize>

impl<T> Pool<T>:
    def new(capacity: usize) -> Pool<T>:
        return Pool {
            blocks: vec![None; capacity],
            free_list: (0..capacity).collect()
        }

    def allocate(&mut self) -> Option<&mut T>:
        if let Some(index) = self.free_list.pop():
            return Some(&mut self.blocks[index])
        return None

    def deallocate(&mut self, index: usize):
        self.blocks[index] = None
        self.free_list.push(index)
```

### 14.6 Using Custom Allocators

```covibe
# Box with custom allocator
let arena = Arena::new(1024)
let boxed = Box::new_in(42, &arena)

# Vec with custom allocator
let mut vec = Vec::new_in(&arena)
vec.push(1)
vec.push(2)
vec.push(3)

# String with custom allocator
let string = String::new_in(&arena)
```

---

## 15. RAII and Drop Order

### 15.1 RAII Pattern

Resource Acquisition Is Initialization: resources are tied to object lifetime:

```covibe
struct File:
    fd: int

impl File:
    def open(path: &str) -> Result<File, IOError>:
        let fd = unsafe { libc::open(path.as_ptr(), O_RDONLY) }
        if fd < 0:
            return Err(IOError)
        return Ok(File { fd })

impl Drop for File:
    def drop(&mut self):
        unsafe:
            libc::close(self.fd)
        print("File closed")

# Usage:
{
    let file = File::open("data.txt")?
    # Use file...
}  # file.drop() called automatically, closes file descriptor
```

### 15.2 The Drop Trait

```covibe
trait Drop:
    def drop(&mut self)
```

**Rules:**
1. `drop` is called automatically when value goes out of scope
2. Cannot call `drop` manually (use `std::mem::drop` to drop early)
3. Drop order is deterministic: reverse order of declaration

### 15.3 Drop Order

**Variables:**
```covibe
def example():
    let x = Resource::new("x")
    let y = Resource::new("y")
    let z = Resource::new("z")
    # Drop order: z, y, x (reverse of declaration)
```

**Struct Fields:**
```covibe
struct Container:
    first: Resource
    second: Resource
    third: Resource

# Drop order: third, second, first (reverse of declaration)
```

**Tuples:**
```covibe
let tuple = (
    Resource::new("a"),
    Resource::new("b"),
    Resource::new("c")
)
# Drop order: c, b, a
```

### 15.4 Drop Flags

The compiler tracks which values need to be dropped:

```covibe
let opt = Some(Resource::new())
if condition:
    let r = opt.unwrap()  # Move out of opt
    # opt is now None, nothing to drop
# If condition is false, opt contains Some, drops the resource
```

**Partial Drops:**
```covibe
struct Pair:
    a: String
    b: String

let p = Pair {
    a: String::from("hello"),
    b: String::from("world")
}

let s = p.a  # Move out p.a
# When p goes out of scope, only p.b is dropped
```

### 15.5 Drop and Copy

Types implementing `Copy` cannot implement `Drop`:

```covibe
@derive(Copy, Clone)
struct Point:
    x: int
    y: int

# ERROR: Cannot implement Drop for Point (it's Copy)
# impl Drop for Point:
#     def drop(&mut self):
#         pass
```

**Reason:** Copy types can be duplicated implicitly, so drop semantics would be unclear.

### 15.6 Early Drop

Use `std::mem::drop` to drop a value early:

```covibe
use std.mem.drop

let resource = acquire_resource()
use_resource(&resource)
drop(resource)  # Explicitly drop here
# resource is now invalid

# Later code that doesn't need resource
```

---

## 16. Defer Statement

### 16.1 Defer Syntax

```ebnf
DeferStatement = 'defer' Expression NEWLINE
```

**Semantics:**
The deferred expression is executed when the enclosing scope ends.

### 16.2 Basic Defer

```covibe
def example():
    let file = open_file("data.txt")
    defer close_file(file)

    # Use file...
    process(file)

    # close_file(file) called here automatically
```

### 16.3 Multiple Defer Statements

Deferred expressions execute in **reverse order** (LIFO):

```covibe
def example():
    defer print("1")
    defer print("2")
    defer print("3")
    print("body")

# Output:
# body
# 3
# 2
# 1
```

### 16.4 Defer vs Drop

**Defer:** Explicit cleanup for any expression
```covibe
let handle = acquire()
defer release(handle)
```

**Drop:** Automatic cleanup for types implementing `Drop`
```covibe
let resource = Resource::new()
# Automatically calls resource.drop() at scope end
```

### 16.5 Defer in Loops

Defer inside a loop executes at the end of each iteration:

```covibe
for i in 0..3:
    defer print(f"End of iteration {i}")
    print(f"Start of iteration {i}")

# Output:
# Start of iteration 0
# End of iteration 0
# Start of iteration 1
# End of iteration 1
# Start of iteration 2
# End of iteration 2
```

### 16.6 Defer Capture

Defer captures variables by value:

```covibe
var x = 1
defer print(x)  # Captures x = 1

x = 2
x = 3
# Prints: 1 (not 3)
```

**To capture by reference, use a closure:**
```covibe
var x = 1
defer { print(x) }  # Captures reference to x

x = 2
x = 3
# Prints: 3
```

---

## 17. Pin and Unpin

### 17.1 The Pin Type

`Pin<P>` prevents moving the pointed-to value:

```covibe
struct Pin<P>:
    pointer: P
```

**Purpose:** Enable self-referential structs for async/await.

### 17.2 Unpin Trait

```covibe
trait Unpin:
    # Auto trait: implemented by default
    # Types that can be safely moved even when pinned
    pass
```

**Almost all types are Unpin:**
- All primitive types
- Most structs and enums
- References

**Not Unpin:**
- Types explicitly opted out with `!Unpin`
- Async futures (often)

### 17.3 Pinning API

```covibe
impl<P> Pin<P>:
    # Safe pinning (requires Unpin)
    def new(pointer: P) -> Pin<P>
    where
        P: Deref,
        P::Target: Unpin:
        return Pin { pointer }

    # Unsafe pinning (works for !Unpin too)
    unsafe def new_unchecked(pointer: P) -> Pin<P>:
        return Pin { pointer }

    # Get immutable reference
    def as_ref(&self) -> Pin<&P::Target>
    where
        P: Deref:
        # ...

    # Get mutable reference (requires Unpin)
    def get_mut(&mut self) -> &mut P::Target
    where
        P: DerefMut,
        P::Target: Unpin:
        # ...
```

### 17.4 Self-Referential Structs

**Problem without Pin:**
```covibe
struct SelfRef:
    data: String
    ptr: *const String  # Points to self.data
```

If `SelfRef` is moved, `ptr` becomes invalid (dangling pointer).

**Solution with Pin:**
```covibe
struct SelfRef:
    data: String
    ptr: *const String

impl !Unpin for SelfRef

# Now SelfRef can only be used through Pin<&SelfRef>
# Moving is prevented
```

### 17.5 Pin in Async

Async futures use Pin to allow self-referential state:

```covibe
async def example():
    let data = fetch_data().await
    process(&data)  # data is borrowed across await
```

**Lowered representation (simplified):**
```covibe
enum ExampleFuture:
    Start
    FetchingData:
        future: FetchFuture
    Processing:
        data: String  # Holds the fetched data
        borrow: *const String  # Self-reference!

impl !Unpin for ExampleFuture
```

### 17.6 Pinning Best Practices

**When to use Pin:**
- Implementing async/await (futures)
- Self-referential structs
- FFI with address-sensitive data

**When NOT to use Pin:**
- Regular synchronous code
- Most data structures
- Anywhere `&T` or `&mut T` works

**Stack Pinning:**
```covibe
use std.pin.pin

let mut value = SomeType::new()
let pinned = pin!(value)
# value is now pinned to the stack
```

**Heap Pinning:**
```covibe
let pinned = Box::pin(SomeType::new())
# Value is pinned in heap-allocated Box
```

---

## Conclusion

This specification defines the complete memory model and ownership system of the CoVibe programming language. The ownership system provides:

- **Memory safety without garbage collection** through compile-time analysis
- **Zero-cost abstractions** that compile to efficient machine code
- **Flexible allocation strategies** supporting stack, heap, and custom allocators
- **Predictable resource management** via RAII and deterministic drop order
- **Advanced features** including Pin for self-referential types and defer for explicit cleanup

The ownership rules, borrow checking, and lifetime system work together to guarantee memory safety while maintaining the performance characteristics of low-level systems programming languages. All safety checks are performed at compile time, ensuring zero runtime overhead.

This memory model serves as the foundation for CoVibe's concurrency guarantees (specified in Part 5) and enables the compiler to generate safe, fast code without runtime memory management overhead.

---

**Document History:**
- 2026-04-14: Initial version 1.0 — Complete memory model and ownership system specification
