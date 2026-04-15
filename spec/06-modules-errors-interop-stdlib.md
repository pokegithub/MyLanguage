# CoVibe Language Specification
## Part 6: Module System, Error Handling, Interop, and Standard Library API

**Version:** 1.0
**Date:** 2026-04-14
**Status:** Final

---

## Table of Contents

1. [Module System](#module-system)
   - 1.1 [Module Declaration](#module-declaration)
   - 1.2 [Import Syntax](#import-syntax)
   - 1.3 [Visibility Rules](#visibility-rules)
   - 1.4 [Cyclic Import Resolution](#cyclic-import-resolution)
   - 1.5 [Package Structure Conventions](#package-structure-conventions)
2. [Error Handling](#error-handling)
   - 2.1 [Result and Option Types](#result-and-option-types)
   - 2.2 [The ? Operator](#the--operator)
   - 2.3 [Try/Catch Sugar](#trycatch-sugar)
   - 2.4 [Panic Semantics](#panic-semantics)
   - 2.5 [Error Context Chaining](#error-context-chaining)
3. [Foreign Function Interface](#foreign-function-interface)
   - 3.1 [C FFI Declaration Syntax](#c-ffi-declaration-syntax)
   - 3.2 [C++ Binding Rules](#c-binding-rules)
   - 3.3 [Python Interop](#python-interop)
   - 3.4 [WebAssembly JavaScript Interop](#webassembly-javascript-interop)
4. [Standard Library API](#standard-library-api)
   - 4.1 [Core Module](#core-module)
   - 4.2 [Collections Module](#collections-module)
   - 4.3 [I/O Module](#io-module)
   - 4.4 [Network Module](#network-module)
   - 4.5 [Concurrency Module](#concurrency-module)
   - 4.6 [Cryptography Module](#cryptography-module)
   - 4.7 [Math Module](#math-module)
   - 4.8 [Time Module](#time-module)
   - 4.9 [Serialization Module](#serialization-module)
   - 4.10 [Testing Module](#testing-module)
   - 4.11 [System Module](#system-module)
   - 4.12 [AI and Machine Learning Module](#ai-and-machine-learning-module)

---

## 1. Module System

The CoVibe module system provides a hierarchical namespace organization that supports code reuse, encapsulation, and dependency management. Modules map to source files and directories, with explicit import and export mechanisms.

### 1.1 Module Declaration

Every CoVibe source file implicitly defines a module. The module's name is derived from its file path relative to the package root.

#### 1.1.1 File-Based Modules

```covibe
# File: src/math/geometry.vibe
# This file automatically defines module `math.geometry`

pub fn calculate_area(width: Float, height: Float) -> Float:
    width * height

fn internal_helper() -> Int:
    42  # Not exported, private to this module
```

#### 1.1.2 Explicit Module Declaration

For inline submodules or to override the default module name:

```covibe
module math.advanced:
    pub fn fast_fourier_transform(data: Array[Complex]) -> Array[Complex]:
        # ... implementation

    fn internal_twiddle_factor(k: Int, n: Int) -> Complex:
        # ... implementation
```

#### 1.1.3 Re-exports

Modules can re-export items from other modules to create a unified public API:

```covibe
# File: src/math/mod.vibe
# Re-export everything from geometry
pub use math.geometry.*

# Re-export specific items with renaming
pub use math.trigonometry.{sin, cos, tan as tangent}

# Re-export a module
pub use math.linear_algebra
```

### 1.2 Import Syntax

Imports bring symbols from other modules into the current scope.

#### 1.2.1 Simple Import

```covibe
import math.geometry
# Usage: math.geometry.calculate_area(10.0, 20.0)
```

#### 1.2.2 Wildcard Import

```covibe
from math.geometry import *
# Usage: calculate_area(10.0, 20.0)
```

**Warning**: Wildcard imports pollute the namespace and should be avoided except in very specific scenarios (e.g., preludes).

#### 1.2.3 Selective Import

```covibe
from math.geometry import calculate_area, calculate_perimeter
# Usage: calculate_area(10.0, 20.0)
```

#### 1.2.4 Aliased Import

```covibe
import math.geometry as geo
# Usage: geo.calculate_area(10.0, 20.0)

from math.trigonometry import sin as sine
# Usage: sine(3.14159)
```

#### 1.2.5 Nested Import

```covibe
from collections import {HashMap, HashSet, BTreeMap.{Entry, OccupiedEntry}}
```

#### 1.2.6 Relative Import

Within a package, modules can use relative imports:

```covibe
# File: src/parser/lexer.vibe
from .tokens import Token, TokenKind  # Same directory
from ..ast import AstNode  # Parent directory
from ...utils import string_utils  # Two levels up
```

#### 1.2.7 External Package Import

```covibe
# Import from an external package declared in covibe.toml
import requests  # Package "requests"
from numpy import array, ndarray  # Python package via FFI
```

### 1.3 Visibility Rules

CoVibe has three visibility levels:

1. **Public (`pub`)**: Accessible from any module
2. **Package (`pkg`)**: Accessible from any module within the same package
3. **Private (default)**: Accessible only within the same module

#### 1.3.1 Public Items

```covibe
pub fn public_function() -> Int:
    42

pub struct PublicStruct:
    pub x: Int        # Public field
    y: Int            # Private field
    pub(pkg) z: Int   # Package-private field
```

#### 1.3.2 Package-Private Items

```covibe
pub(pkg) fn package_only_function() -> String:
    "Only accessible within this package"

pub(pkg) type InternalAlias = HashMap[String, Int]
```

#### 1.3.3 Private Items

```covibe
fn private_function() -> Bool:
    true

struct PrivateStruct:
    data: Int
```

#### 1.3.4 Visibility Inheritance

- Struct fields inherit the struct's visibility unless explicitly specified
- Enum variants are always as visible as the enum itself
- Trait methods are as visible as the trait
- Implementation items (`impl` blocks) can have independent visibility

```covibe
pub struct Point:
    pub x: Float  # Public field
    pub y: Float  # Public field

pub enum Color:
    Red           # Public variant (same as enum)
    Green         # Public variant
    Blue          # Public variant

pub trait Drawable:
    pub fn draw(self) -> None  # Public method
```

#### 1.3.5 Visibility Rules for Types

A type can only be returned or accepted by a function if it is at least as visible as the function:

```covibe
# Error: private type in public interface
struct PrivateData:
    value: Int

pub fn get_data() -> PrivateData:  # ❌ Compile error
    PrivateData { value: 42 }
```

### 1.4 Cyclic Import Resolution

CoVibe supports cyclic module dependencies but enforces strict rules to prevent initialization deadlocks.

#### 1.4.1 Type-Only Cycles

Type definitions can be mutually recursive across modules:

```covibe
# File: ast/expr.vibe
from .stmt import Statement

pub enum Expression:
    Block(statements: Array[Statement])
    # ... other variants

# File: ast/stmt.vibe
from .expr import Expression

pub enum Statement:
    Expr(expr: Expression)
    # ... other variants
```

**Resolution**: The compiler builds a module dependency graph and performs topological sorting. Type-only cycles are allowed because type information is resolved before value initialization.

#### 1.4.2 Value Cycles

Value-level cyclic dependencies are forbidden and result in a compile-time error:

```covibe
# File: a.vibe
from b import B_VALUE

pub A_VALUE: Int = B_VALUE + 1

# File: b.vibe
from a import A_VALUE

pub B_VALUE: Int = A_VALUE + 1

# ❌ Compile error: cyclic value initialization between a and b
```

#### 1.4.3 Cycle Detection Algorithm

The compiler uses Tarjan's strongly connected components algorithm to detect cycles in the module dependency graph:

1. Build a directed graph where nodes are modules and edges are imports
2. Find all strongly connected components (SCCs)
3. For each SCC with more than one node:
   - If any edge in the cycle is a value dependency → **Error**
   - If all edges are type-only dependencies → **Allowed**

#### 1.4.4 Breaking Cycles

To break value cycles, use lazy initialization or indirection:

```covibe
# File: a.vibe
from b import get_b_value

pub A_VALUE: Int = 10

pub fn get_a_value() -> Int:
    A_VALUE

# File: b.vibe
from a import get_a_value

pub B_VALUE: Int = 20

pub fn get_b_value() -> Int:
    B_VALUE
```

### 1.5 Package Structure Conventions

A CoVibe package follows a standard directory layout:

```
my_package/
├── covibe.toml           # Package manifest
├── README.md             # Package documentation
├── LICENSE               # License file
├── src/                  # Source code root
│   ├── lib.vibe         # Library entry point (for libraries)
│   ├── main.vibe        # Binary entry point (for executables)
│   ├── module_a.vibe    # Module: my_package.module_a
│   ├── module_b/         # Submodule directory
│   │   ├── mod.vibe     # Module: my_package.module_b
│   │   ├── sub1.vibe    # Module: my_package.module_b.sub1
│   │   └── sub2.vibe    # Module: my_package.module_b.sub2
│   └── prelude.vibe     # Prelude module (auto-imported)
├── tests/                # Integration tests
│   ├── test_feature_a.vibe
│   └── test_feature_b.vibe
├── benches/              # Benchmarks
│   └── benchmark_main.vibe
├── examples/             # Example programs
│   └── example_usage.vibe
└── docs/                 # Documentation
    └── guide.md
```

#### 1.5.1 Package Manifest (covibe.toml)

```toml
[package]
name = "my_package"
version = "1.0.0"
authors = ["Jane Doe <jane@example.com>"]
description = "A description of my package"
license = "MIT"
repository = "https://github.com/user/my_package"
keywords = ["parser", "compiler", "ast"]
categories = ["development-tools"]
edition = "2026"

[dependencies]
regex = "2.0"
serde = { version = "3.0", features = ["derive"] }

[dev-dependencies]
proptest = "1.0"

[build-dependencies]
bindgen = "0.5"

[[bin]]
name = "my_tool"
path = "src/main.vibe"

[lib]
name = "my_library"
path = "src/lib.vibe"

[features]
default = ["std"]
std = []
no_std = []
experimental = []

[profile.release]
opt_level = 3
lto = true
codegen_units = 1
```

#### 1.5.2 Module Path Resolution

Module paths are resolved as follows:

1. `src/lib.vibe` or `src/main.vibe` defines the root module
2. `src/foo.vibe` defines module `foo`
3. `src/foo/mod.vibe` also defines module `foo`
4. `src/foo/bar.vibe` defines module `foo.bar`
5. External packages are resolved via `covibe.toml` dependencies

#### 1.5.3 Prelude

The prelude module (`std.prelude` for standard library, or `my_package.prelude` for packages) is automatically imported into every module:

```covibe
# File: src/prelude.vibe
# This module is automatically imported as: from my_package.prelude import *

pub use core.{Option, Result, Some, None, Ok, Err}
pub use collections.{Vec, HashMap, HashSet}
pub use io.{print, println, eprint, eprintln}
```

To disable auto-import of the prelude:

```covibe
#![no_implicit_prelude]

# Must manually import everything
from std.core import Option, Result
```

---

## 2. Error Handling

CoVibe uses a dual error handling strategy:

1. **Recoverable errors**: Represented using `Result<T, E>` and `Option<T>` types
2. **Unrecoverable errors**: Represented using panics

This design allows for explicit, type-safe error propagation while still providing a mechanism for fatal failures.

### 2.1 Result and Option Types

#### 2.1.1 Option Type

The `Option<T>` type represents an optional value:

```covibe
enum Option[T]:
    Some(value: T)
    None
```

**Common operations:**

```covibe
# Pattern matching
match maybe_value:
    Some(x) -> println("Got value: {x}")
    None -> println("No value")

# Unwrapping (panics if None)
value = maybe_value.unwrap()

# Unwrapping with default
value = maybe_value.unwrap_or(42)

# Unwrapping with lazy default
value = maybe_value.unwrap_or_else(|| expensive_computation())

# Map operation
result = maybe_value.map(|x| x * 2)

# And then (flat map)
result = maybe_value.and_then(|x| Some(x * 2) if x > 0 else None)

# Or else
result = maybe_value.or_else(|| Some(42))

# Filtering
result = maybe_value.filter(|x| x > 10)

# Take ownership and replace with None
value = maybe_value.take()

# Replace value and return old value
old_value = maybe_value.replace(new_value)
```

#### 2.1.2 Result Type

The `Result<T, E>` type represents either success (`Ok`) or failure (`Err`):

```covibe
enum Result[T, E]:
    Ok(value: T)
    Err(error: E)
```

**Common operations:**

```covibe
# Pattern matching
match result:
    Ok(value) -> println("Success: {value}")
    Err(error) -> println("Error: {error}")

# Unwrapping (panics if Err)
value = result.unwrap()

# Unwrapping error (panics if Ok)
error = result.unwrap_err()

# Expect (unwrap with custom panic message)
value = result.expect("Failed to parse config")

# Unwrap with default
value = result.unwrap_or(42)

# Map success value
result = result.map(|x| x * 2)

# Map error value
result = result.map_err(|e| format("Error: {e}"))

# And then (flat map for success)
result = result.and_then(|x| Ok(x * 2) if x > 0 else Err("Negative value"))

# Or else (flat map for error)
result = result.or_else(|e| Ok(42))

# Check if Ok
if result.is_ok():
    # ...

# Check if Err
if result.is_err():
    # ...

# Convert to Option
maybe_value = result.ok()
maybe_error = result.err()
```

#### 2.1.3 Type Aliases

Common error result types have short aliases:

```covibe
# Standard library Result with std::Error
type StdResult[T] = Result[T, Error]

# I/O specific Result
type IoResult[T] = Result[T, IoError]

# Parse specific Result
type ParseResult[T] = Result[T, ParseError]
```

### 2.2 The ? Operator

The `?` operator provides syntactic sugar for early return on error:

```covibe
fn read_file_to_string(path: String) -> Result[String, IoError]:
    file = File.open(path)?  # Returns Err early if open fails
    content = file.read_to_string()?  # Returns Err early if read fails
    Ok(content)
```

**Desugaring:**

The `?` operator desugars to:

```covibe
# Expression: value?
# Desugars to:
match expression:
    Ok(val) -> val
    Err(err) -> return Err(err.into())
```

The `.into()` call allows automatic error type conversion if the current function's error type implements `From<SourceError>`.

#### 2.2.1 ? Operator with Option

The `?` operator also works with `Option`:

```covibe
fn get_first_word(text: String) -> Option[String]:
    first_char = text.chars().next()?  # Returns None if empty
    word = text.split_whitespace().next()?  # Returns None if no words
    Some(word.to_string())
```

#### 2.2.2 Try Blocks

For handling multiple fallible operations in a single expression:

```covibe
result = try:
    file = File.open("data.txt")?
    content = file.read_to_string()?
    parsed = parse_json(content)?
    parsed.get("key")?

# `result` has type Result[T, E] where T is the type of the last expression
```

### 2.3 Try/Catch Sugar

For ease-of-use, CoVibe provides `try/catch` syntax as sugar over `Result`:

```covibe
try:
    file = File.open("data.txt")
    content = file.read_to_string()
    data = parse_json(content)
    println("Data: {data}")
catch e as IoError:
    eprintln("I/O error: {e}")
catch e as JsonError:
    eprintln("JSON parsing error: {e}")
catch e:
    eprintln("Unknown error: {e}")
finally:
    println("Cleanup code here")
```

**Desugaring:**

```covibe
# Try/catch desugars to:
__try_result = (|| -> Result[(), Error]:
    file = File.open("data.txt")?
    content = file.read_to_string()?
    data = parse_json(content)?
    println("Data: {data}")
    Ok(())
)()

match __try_result:
    Err(e) if e.is::<IoError>() ->
        let e = e.downcast::<IoError>()
        eprintln("I/O error: {e}")
    Err(e) if e.is::<JsonError>() ->
        let e = e.downcast::<JsonError>()
        eprintln("JSON parsing error: {e}")
    Err(e) ->
        eprintln("Unknown error: {e}")
    Ok(_) ->
        ()

# Finally clause always executes
println("Cleanup code here")
```

**Important**: `try/catch` is purely syntactic sugar. All errors remain type-checked at compile time, and the `Error` trait must be implemented by error types.

#### 2.3.1 Type-Checked Exceptions

CoVibe tracks possible exceptions at the type level using effect types (see Part 6 for details):

```covibe
fn risky_operation() -> Int throws IoError | ParseError:
    # Function signature declares it may throw IoError or ParseError
    file = File.open("data.txt")  # May throw IoError
    parse_int(file.read_to_string())  # May throw ParseError
```

Callers must handle all declared exception types:

```covibe
try:
    result = risky_operation()
catch e as IoError:
    # Must handle IoError
catch e as ParseError:
    # Must handle ParseError
# No catch-all needed if all types are handled
```

### 2.4 Panic Semantics

A panic represents an unrecoverable error that terminates the current task/thread. Panics should be used for:

- Programming bugs (assertion failures, out-of-bounds access)
- Unrecoverable resource exhaustion
- Situations where recovery is impossible or meaningless

#### 2.4.1 Creating a Panic

```covibe
panic("Something went terribly wrong")
panic("Index {index} out of bounds for length {length}")
```

#### 2.4.2 Assertion Macros

```covibe
assert(condition)
assert(condition, "Custom message")
assert_eq(left, right)
assert_eq(left, right, "Values not equal")
assert_ne(left, right)
debug_assert(condition)  # Only checked in debug builds
```

#### 2.4.3 Unwinding vs. Aborting

CoVibe supports two panic strategies:

1. **Unwinding (default)**: The stack is unwound, running destructors (RAII cleanup)
2. **Aborting**: The process immediately terminates without cleanup

Configured in `covibe.toml`:

```toml
[profile.release]
panic = "abort"  # or "unwind"
```

#### 2.4.4 Catching Panics

Panics can be caught (for resilience, not control flow):

```covibe
result = catch_panic(|| {
    # Code that might panic
    dangerous_operation()
})

match result:
    Ok(value) -> println("Succeeded: {value}")
    Err(panic_info) -> eprintln("Caught panic: {panic_info}")
```

**Important**: `catch_panic` should only be used at architectural boundaries (e.g., isolating plugin code, server request handlers). Never use it for normal control flow.

#### 2.4.5 Stack Traces

Panics automatically capture and print a stack trace (in debug mode):

```
thread 'main' panicked at 'assertion failed: x < 10', src/main.vibe:42:5
stack backtrace:
   0: std::panic::panic
   1: my_app::validate_input
   2: my_app::main
```

Stack traces can be disabled for release builds:

```toml
[profile.release]
debug = false
```

### 2.5 Error Context Chaining

CoVibe supports wrapping errors with additional context:

#### 2.5.1 Context Trait

```covibe
trait Context[T, E]:
    fn context(self, msg: String) -> Result[T, ContextError[E]]
    fn with_context[F](self, f: F) -> Result[T, ContextError[E]]
        where F: FnOnce() -> String
```

**Usage:**

```covibe
fn load_config() -> Result[Config, Error]:
    content = File.open("config.toml")
        .context("Failed to open config file")?

    config = parse_toml(content)
        .with_context(|| format("Failed to parse config at line {line}"))?

    Ok(config)
```

#### 2.5.2 Error Chain

Errors can be chained to preserve the full causal chain:

```covibe
match load_config():
    Ok(config) -> # ...
    Err(err) ->
        # Print the full error chain
        eprintln("Error: {err}")

        mut source = err.source()
        while source.is_some():
            eprintln("  Caused by: {source.unwrap()}")
            source = source.unwrap().source()
```

#### 2.5.3 Custom Error Types

```covibe
use std.error.Error
use std.fmt.{Display, Formatter, Result as FmtResult}

pub struct ConfigError:
    message: String
    line: Option[Int]
    source: Option[Box[dyn Error]]

impl Display for ConfigError:
    fn fmt(self, f: Formatter) -> FmtResult:
        if let Some(line) = self.line:
            write(f, "Config error at line {line}: {self.message}")
        else:
            write(f, "Config error: {self.message}")

impl Error for ConfigError:
    fn source(self) -> Option[&dyn Error]:
        self.source.as_ref().map(|e| e.as_ref())
```

---

## 3. Foreign Function Interface

CoVibe provides comprehensive FFI support for interoperating with C, C++, Python, and JavaScript (via WebAssembly).

### 3.1 C FFI Declaration Syntax

#### 3.1.1 Declaring External C Functions

```covibe
extern "C":
    fn malloc(size: usize) -> *mut u8
    fn free(ptr: *mut u8) -> ()
    fn strlen(s: *const u8) -> usize
    fn printf(format: *const u8, ...) -> i32
```

#### 3.1.2 Calling C Functions

C functions are inherently unsafe and must be called in `unsafe` blocks:

```covibe
unsafe:
    ptr = malloc(1024)
    if ptr.is_null():
        panic("Allocation failed")

    # Use ptr...

    free(ptr)
```

#### 3.1.3 ABI Specification

CoVibe supports multiple ABIs:

```covibe
extern "C":        # C ABI (default)
extern "cdecl":    # C calling convention (Windows x86)
extern "stdcall":  # Windows stdcall
extern "fastcall": # Windows fastcall
extern "win64":    # Windows x64
extern "sysv64":   # System V AMD64 ABI (Unix/Linux)
extern "rust":     # Rust ABI (unstable)
```

#### 3.1.4 Type Mappings

| CoVibe Type | C Type | Notes |
|-------------|--------|-------|
| `i8` | `int8_t` | |
| `i16` | `int16_t` | |
| `i32` | `int32_t` | |
| `i64` | `int64_t` | |
| `u8` | `uint8_t` | |
| `u16` | `uint16_t` | |
| `u32` | `uint32_t` | |
| `u64` | `uint64_t` | |
| `isize` | `intptr_t` | Platform-dependent |
| `usize` | `size_t` | Platform-dependent |
| `f32` | `float` | |
| `f64` | `double` | |
| `bool` | `bool` (C99) | Represented as `u8` |
| `*const T` | `const T*` | Raw pointer |
| `*mut T` | `T*` | Mutable raw pointer |
| `()` | `void` | |
| `#[repr(C)] struct` | `struct` | C-compatible layout |

#### 3.1.5 repr Attribute

To ensure struct layout compatibility with C:

```covibe
#[repr(C)]
struct Point:
    x: f64
    y: f64

#[repr(C)]
enum Status:
    Ok = 0
    Error = 1
    Pending = 2

#[repr(C, packed)]
struct PackedData:
    flag: u8
    value: u32
```

#### 3.1.6 Exporting Functions to C

```covibe
#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i32:
    a + b

#[export_name = "custom_name"]
pub extern "C" fn internal_func() -> ():
    # ...
```

#### 3.1.7 Static Variables

```covibe
extern "C":
    static GLOBAL_VAR: i32
    static mut MUTABLE_GLOBAL: *mut u8

unsafe:
    value = GLOBAL_VAR
    MUTABLE_GLOBAL = malloc(100)
```

#### 3.1.8 Variadic Functions

CoVibe supports calling C variadic functions but not defining them:

```covibe
extern "C":
    fn printf(format: *const u8, ...) -> i32

unsafe:
    printf(c"Hello %s, value: %d\n", c"World", 42)
```

#### 3.1.9 C String Literals

```covibe
# C string literal (null-terminated)
c_str = c"Hello, World!"
# Type: &'static CStr (null-terminated, no interior nulls)

# Convert to C pointer
ptr = c_str.as_ptr()  # Type: *const u8
```

### 3.2 C++ Binding Rules

C++ interop is more complex due to name mangling, templates, and classes. CoVibe provides automatic binding generation.

#### 3.2.1 Automatic Binding Generation

```covibe
#[cxx::bridge]
mod ffi:
    extern "C++":
        include!("mylib.hpp")

        type MyClass

        fn create_instance() -> UniquePtr[MyClass]
        fn process(self: Pin[&mut MyClass], data: &[u8]) -> Result[()]
        fn get_value(self: &MyClass) -> i32
```

#### 3.2.2 C++ Types in CoVibe

| C++ Type | CoVibe Type |
|----------|-------------|
| `std::unique_ptr<T>` | `UniquePtr[T]` |
| `std::shared_ptr<T>` | `SharedPtr[T]` |
| `std::string` | `CxxString` |
| `std::vector<T>` | `CxxVector[T]` |
| `T&` | `&T` or `Pin[&mut T]` |
| `const T&` | `&T` |

#### 3.2.3 Calling C++ Methods

```covibe
obj = ffi.create_instance()
value = obj.get_value()
obj.process(data.as_bytes())?
```

#### 3.2.4 Exception Handling

C++ exceptions are automatically converted to `Result`:

```covibe
extern "C++":
    fn may_throw() -> Result[i32]  # C++ exceptions -> Result

match may_throw():
    Ok(value) -> println("Success: {value}")
    Err(e) -> eprintln("C++ exception: {e}")
```

### 3.3 Python Interop

CoVibe can embed a Python interpreter and call Python code bidirectionally.

#### 3.3.1 Importing Python Modules

```covibe
use std.python.{Python, PyObject, PyModule}

py = Python.acquire_gil()
np = py.import("numpy")?

# Call Python function
array = np.call("array", ([1, 2, 3, 4],))?
result = np.call("sum", (array,))?

println("Sum: {result.extract::<i64>()?}")
```

#### 3.3.2 Type Conversions

| CoVibe Type | Python Type |
|-------------|-------------|
| `i32`, `i64` | `int` |
| `f32`, `f64` | `float` |
| `String` | `str` |
| `bool` | `bool` |
| `Vec[T]` | `list` |
| `HashMap[K, V]` | `dict` |
| `Option[T]` | `T` or `None` |
| `Result[T, E]` | `T` or raises exception |

#### 3.3.3 Calling Python Functions

```covibe
fn call_python_function(py: &Python) -> Result[i64]:
    # Import module
    module = py.import("math")?

    # Get function
    sqrt_fn = module.getattr("sqrt")?

    # Call with arguments
    result = sqrt_fn.call1((16.0,))?

    # Extract result
    Ok(result.extract::<f64>()? as i64)
```

#### 3.3.4 Exposing CoVibe Functions to Python

```covibe
#[pyfunction]
fn add(a: i64, b: i64) -> i64:
    a + b

#[pymodule]
fn my_module(py: Python, module: &PyModule) -> PyResult[()]:
    module.add_function(wrap_pyfunction!(add, module)?)?
    Ok(())
```

Usage from Python:

```python
import my_module
result = my_module.add(10, 20)
```

#### 3.3.5 Python Classes in CoVibe

```covibe
#[pyclass]
struct Counter:
    #[pyo3(get, set)]
    value: i64

#[pymethods]
impl Counter:
    #[new]
    fn new(initial: i64) -> Self:
        Counter { value: initial }

    fn increment(&mut self) -> ():
        self.value += 1

    fn get_value(&self) -> i64:
        self.value
```

### 3.4 WebAssembly JavaScript Interop

When compiled to WebAssembly, CoVibe can interoperate with JavaScript.

#### 3.4.1 Importing JavaScript Functions

```covibe
#[wasm_bindgen]
extern "C":
    fn alert(s: &str)

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str)

    #[wasm_bindgen(js_namespace = Math)]
    fn random() -> f64

# Usage:
alert("Hello from CoVibe!")
log("Debug message")
value = random()
```

#### 3.4.2 Exporting to JavaScript

```covibe
#[wasm_bindgen]
pub fn greet(name: &str) -> String:
    format("Hello, {name}!")

#[wasm_bindgen]
pub struct Calculator:
    value: f64

#[wasm_bindgen]
impl Calculator:
    #[wasm_bindgen(constructor)]
    pub fn new() -> Calculator:
        Calculator { value: 0.0 }

    pub fn add(&mut self, x: f64) -> ():
        self.value += x

    pub fn get_value(&self) -> f64:
        self.value
```

JavaScript usage:

```javascript
import init, { greet, Calculator } from './my_wasm_module.js';

await init();

console.log(greet('World'));

const calc = new Calculator();
calc.add(10);
calc.add(20);
console.log(calc.get_value()); // 30
```

#### 3.4.3 JavaScript Types

```covibe
use wasm_bindgen.JsValue

#[wasm_bindgen]
pub fn process_js_value(val: JsValue) -> JsValue:
    # Can work with any JavaScript value
    val
```

#### 3.4.4 DOM Access

```covibe
use web_sys.{Document, Element, HtmlElement, Window}

fn get_document() -> Result[Document]:
    window = web_sys.window().ok_or("No window")?
    Ok(window.document().ok_or("No document")?)

fn create_button(text: &str) -> Result[()]:
    document = get_document()?
    button = document.create_element("button")?
    button.set_text_content(Some(text))

    body = document.body().ok_or("No body")?
    body.append_child(&button)?

    Ok(())
```

---

## 4. Standard Library API

The CoVibe standard library provides comprehensive functionality organized into modules.

### 4.1 Core Module

**Module:** `std.core`

The core module contains fundamental types and traits that are auto-imported via the prelude.

#### 4.1.1 Option and Result

```covibe
enum Option[T]:
    Some(T)
    None

enum Result[T, E]:
    Ok(T)
    Err(E)
```

#### 4.1.2 Basic Traits

```covibe
trait Copy:
    # Marker trait for types that can be copied bitwise

trait Clone:
    fn clone(self: &Self) -> Self

trait Drop:
    fn drop(self: &mut Self) -> ()

trait Default:
    fn default() -> Self

trait From[T]:
    fn from(value: T) -> Self

trait Into[T]:
    fn into(self) -> T

trait Display:
    fn fmt(self: &Self, f: &mut Formatter) -> Result[(), FmtError]

trait Debug:
    fn fmt(self: &Self, f: &mut Formatter) -> Result[(), FmtError]
```

#### 4.1.3 Comparison Traits

```covibe
trait PartialEq[Rhs = Self]:
    fn eq(self: &Self, other: &Rhs) -> bool
    fn ne(self: &Self, other: &Rhs) -> bool

trait Eq: PartialEq[Self]
    # Marker trait for full equivalence

trait PartialOrd[Rhs = Self]: PartialEq[Rhs]:
    fn partial_cmp(self: &Self, other: &Rhs) -> Option[Ordering]

trait Ord: Eq + PartialOrd[Self]:
    fn cmp(self: &Self, other: &Self) -> Ordering

enum Ordering:
    Less
    Equal
    Greater
```

#### 4.1.4 Operators

```covibe
trait Add[Rhs = Self]:
    type Output
    fn add(self, rhs: Rhs) -> Self.Output

trait Sub[Rhs = Self]:
    type Output
    fn sub(self, rhs: Rhs) -> Self.Output

trait Mul[Rhs = Self]:
    type Output
    fn mul(self, rhs: Rhs) -> Self.Output

trait Div[Rhs = Self]:
    type Output
    fn div(self, rhs: Rhs) -> Self.Output

trait Rem[Rhs = Self]:
    type Output
    fn rem(self, rhs: Rhs) -> Self.Output

# Bitwise operators
trait BitAnd[Rhs = Self]:
    type Output
    fn bitand(self, rhs: Rhs) -> Self.Output

trait BitOr[Rhs = Self]:
    type Output
    fn bitor(self, rhs: Rhs) -> Self.Output

trait BitXor[Rhs = Self]:
    type Output
    fn bitxor(self, rhs: Rhs) -> Self.Output

trait Not:
    type Output
    fn not(self) -> Self.Output

trait Shl[Rhs]:
    type Output
    fn shl(self, rhs: Rhs) -> Self.Output

trait Shr[Rhs]:
    type Output
    fn shr(self, rhs: Rhs) -> Self.Output

# Index operators
trait Index[Idx]:
    type Output
    fn index(self: &Self, index: Idx) -> &Self.Output

trait IndexMut[Idx]: Index[Idx]:
    fn index_mut(self: &mut Self, index: Idx) -> &mut Self.Output
```

#### 4.1.5 Smart Pointers

```covibe
struct Box[T]:
    # Heap-allocated, owned pointer
    fn new(value: T) -> Box[T]
    fn leak(self) -> &'static mut T

struct Rc[T]:
    # Reference-counted pointer
    fn new(value: T) -> Rc[T]
    fn strong_count(self: &Self) -> usize
    fn weak_count(self: &Self) -> usize
    fn try_unwrap(self) -> Result[T, Rc[T]]

struct Arc[T]:
    # Atomic reference-counted pointer (thread-safe)
    fn new(value: T) -> Arc[T]
    fn strong_count(self: &Self) -> usize
    fn weak_count(self: &Self) -> usize
    fn try_unwrap(self) -> Result[T, Arc[T]]

struct Weak[T]:
    # Weak reference (doesn't prevent deallocation)
    fn upgrade(self: &Self) -> Option[Rc[T]]
```

#### 4.1.6 Cell Types (Interior Mutability)

```covibe
struct Cell[T]:
    fn new(value: T) -> Cell[T]
    fn get(self: &Self) -> T where T: Copy
    fn set(self: &Self, value: T) -> ()
    fn replace(self: &Self, value: T) -> T
    fn take(self: &Self) -> T where T: Default

struct RefCell[T]:
    fn new(value: T) -> RefCell[T]
    fn borrow(self: &Self) -> Ref[T]
    fn borrow_mut(self: &Self) -> RefMut[T]
    fn try_borrow(self: &Self) -> Result[Ref[T], BorrowError]
    fn try_borrow_mut(self: &Self) -> Result[RefMut[T], BorrowMutError]
```

### 4.2 Collections Module

**Module:** `std.collections`

#### 4.2.1 Vec (Dynamic Array)

```covibe
struct Vec[T]:
    fn new() -> Vec[T]
    fn with_capacity(capacity: usize) -> Vec[T]
    fn push(self: &mut Self, value: T) -> ()
    fn pop(self: &mut Self) -> Option[T]
    fn insert(self: &mut Self, index: usize, value: T) -> ()
    fn remove(self: &mut Self, index: usize) -> T
    fn clear(self: &mut Self) -> ()
    fn len(self: &Self) -> usize
    fn is_empty(self: &Self) -> bool
    fn capacity(self: &Self) -> usize
    fn reserve(self: &mut Self, additional: usize) -> ()
    fn shrink_to_fit(self: &mut Self) -> ()
    fn truncate(self: &mut Self, len: usize) -> ()
    fn swap_remove(self: &mut Self, index: usize) -> T
    fn append(self: &mut Self, other: &mut Vec[T]) -> ()
    fn drain(self: &mut Self, range: Range[usize]) -> Drain[T]
    fn retain(self: &mut Self, f: impl Fn(&T) -> bool) -> ()
    fn dedup(self: &mut Self) -> () where T: PartialEq
    fn sort(self: &mut Self) -> () where T: Ord
    fn sort_by(self: &mut Self, compare: impl Fn(&T, &T) -> Ordering) -> ()
    fn binary_search(self: &Self, x: &T) -> Result[usize, usize] where T: Ord
```

#### 4.2.2 HashMap

```covibe
struct HashMap[K, V]:
    fn new() -> HashMap[K, V]
    fn with_capacity(capacity: usize) -> HashMap[K, V]
    fn insert(self: &mut Self, key: K, value: V) -> Option[V]
    fn get(self: &Self, key: &K) -> Option[&V]
    fn get_mut(self: &mut Self, key: &K) -> Option[&mut V]
    fn remove(self: &mut Self, key: &K) -> Option[V]
    fn contains_key(self: &Self, key: &K) -> bool
    fn clear(self: &mut Self) -> ()
    fn len(self: &Self) -> usize
    fn is_empty(self: &Self) -> bool
    fn keys(self: &Self) -> Keys[K, V]
    fn values(self: &Self) -> Values[K, V]
    fn values_mut(self: &mut Self) -> ValuesMut[K, V]
    fn iter(self: &Self) -> Iter[K, V]
    fn iter_mut(self: &mut Self) -> IterMut[K, V]
    fn entry(self: &mut Self, key: K) -> Entry[K, V]
```

#### 4.2.3 HashSet

```covibe
struct HashSet[T]:
    fn new() -> HashSet[T]
    fn with_capacity(capacity: usize) -> HashSet[T]
    fn insert(self: &mut Self, value: T) -> bool
    fn remove(self: &mut Self, value: &T) -> bool
    fn contains(self: &Self, value: &T) -> bool
    fn clear(self: &mut Self) -> ()
    fn len(self: &Self) -> usize
    fn is_empty(self: &Self) -> bool
    fn iter(self: &Self) -> Iter[T]
    fn union(self: &Self, other: &HashSet[T]) -> Union[T]
    fn intersection(self: &Self, other: &HashSet[T]) -> Intersection[T]
    fn difference(self: &Self, other: &HashSet[T]) -> Difference[T]
    fn symmetric_difference(self: &Self, other: &HashSet[T]) -> SymmetricDifference[T]
    fn is_subset(self: &Self, other: &HashSet[T]) -> bool
    fn is_superset(self: &Self, other: &HashSet[T]) -> bool
```

#### 4.2.4 BTreeMap

```covibe
struct BTreeMap[K, V]:
    fn new() -> BTreeMap[K, V]
    fn insert(self: &mut Self, key: K, value: V) -> Option[V]
    fn get(self: &Self, key: &K) -> Option[&V]
    fn remove(self: &mut Self, key: &K) -> Option[V]
    fn range(self: &Self, range: impl RangeBounds[K]) -> Range[K, V]
    fn first_key_value(self: &Self) -> Option[(&K, &V)]
    fn last_key_value(self: &Self) -> Option[(&K, &V)]
    fn pop_first(self: &mut Self) -> Option[(K, V)]
    fn pop_last(self: &mut Self) -> Option[(K, V)]
```

#### 4.2.5 BTreeSet

```covibe
struct BTreeSet[T]:
    fn new() -> BTreeSet[T]
    fn insert(self: &mut Self, value: T) -> bool
    fn remove(self: &mut Self, value: &T) -> bool
    fn contains(self: &Self, value: &T) -> bool
    fn range(self: &Self, range: impl RangeBounds[T]) -> Range[T]
    fn first(self: &Self) -> Option[&T]
    fn last(self: &Self) -> Option[&T]
    fn pop_first(self: &mut Self) -> Option[T]
    fn pop_last(self: &mut Self) -> Option[T]
```

#### 4.2.6 LinkedList

```covibe
struct LinkedList[T]:
    fn new() -> LinkedList[T]
    fn push_front(self: &mut Self, value: T) -> ()
    fn push_back(self: &mut Self, value: T) -> ()
    fn pop_front(self: &mut Self) -> Option[T]
    fn pop_back(self: &mut Self) -> Option[T]
    fn front(self: &Self) -> Option[&T]
    fn back(self: &Self) -> Option[&T]
    fn len(self: &Self) -> usize
    fn is_empty(self: &Self) -> bool
    fn clear(self: &mut Self) -> ()
```

#### 4.2.7 VecDeque (Double-ended queue)

```covibe
struct VecDeque[T]:
    fn new() -> VecDeque[T]
    fn with_capacity(capacity: usize) -> VecDeque[T]
    fn push_front(self: &mut Self, value: T) -> ()
    fn push_back(self: &mut Self, value: T) -> ()
    fn pop_front(self: &mut Self) -> Option[T]
    fn pop_back(self: &mut Self) -> Option[T]
    fn front(self: &Self) -> Option[&T]
    fn back(self: &Self) -> Option[&T]
    fn len(self: &Self) -> usize
    fn is_empty(self: &Self) -> bool
```

#### 4.2.8 PriorityQueue (Binary Heap)

```covibe
struct PriorityQueue[T]:
    fn new() -> PriorityQueue[T]
    fn with_capacity(capacity: usize) -> PriorityQueue[T]
    fn push(self: &mut Self, value: T) -> ()
    fn pop(self: &mut Self) -> Option[T]
    fn peek(self: &Self) -> Option[&T]
    fn len(self: &Self) -> usize
    fn is_empty(self: &Self) -> bool
    fn clear(self: &mut Self) -> ()
```

### 4.3 I/O Module

**Module:** `std.io`

#### 4.3.1 Core Traits

```covibe
trait Read:
    fn read(self: &mut Self, buf: &mut [u8]) -> Result[usize, IoError]
    fn read_to_end(self: &mut Self, buf: &mut Vec[u8]) -> Result[usize, IoError]
    fn read_to_string(self: &mut Self, buf: &mut String) -> Result[usize, IoError]
    fn read_exact(self: &mut Self, buf: &mut [u8]) -> Result[(), IoError]

trait Write:
    fn write(self: &mut Self, buf: &[u8]) -> Result[usize, IoError]
    fn write_all(self: &mut Self, buf: &[u8]) -> Result[(), IoError]
    fn flush(self: &mut Self) -> Result[(), IoError]
    fn write_fmt(self: &mut Self, fmt: Arguments) -> Result[(), IoError]

trait Seek:
    fn seek(self: &mut Self, pos: SeekFrom) -> Result[u64, IoError]
    fn stream_position(self: &mut Self) -> Result[u64, IoError]
```

#### 4.3.2 File Operations

```covibe
struct File:
    fn open(path: impl AsRef[Path]) -> Result[File, IoError]
    fn create(path: impl AsRef[Path]) -> Result[File, IoError]
    fn options() -> OpenOptions
    fn sync_all(self: &Self) -> Result[(), IoError]
    fn sync_data(self: &Self) -> Result[(), IoError]
    fn set_len(self: &Self, size: u64) -> Result[(), IoError]
    fn metadata(self: &Self) -> Result[Metadata, IoError]
    fn try_clone(self: &Self) -> Result[File, IoError]

struct OpenOptions:
    fn new() -> OpenOptions
    fn read(self: &mut Self, read: bool) -> &mut Self
    fn write(self: &mut Self, write: bool) -> &mut Self
    fn append(self: &mut Self, append: bool) -> &mut Self
    fn truncate(self: &mut Self, truncate: bool) -> &mut Self
    fn create(self: &mut Self, create: bool) -> &mut Self
    fn create_new(self: &mut Self, create_new: bool) -> &mut Self
    fn open(self: &Self, path: impl AsRef[Path]) -> Result[File, IoError]
```

#### 4.3.3 Buffered I/O

```covibe
struct BufReader[R]:
    fn new(inner: R) -> BufReader[R]
    fn with_capacity(capacity: usize, inner: R) -> BufReader[R]
    fn buffer(self: &Self) -> &[u8]
    fn capacity(self: &Self) -> usize

struct BufWriter[W]:
    fn new(inner: W) -> BufWriter[W]
    fn with_capacity(capacity: usize, inner: W) -> BufWriter[W]
    fn buffer(self: &Self) -> &[u8]
    fn capacity(self: &Self) -> usize
```

#### 4.3.4 Standard Streams

```covibe
fn stdin() -> Stdin
fn stdout() -> Stdout
fn stderr() -> Stderr

# Print functions
fn print(args: Arguments) -> ()
fn println(args: Arguments) -> ()
fn eprint(args: Arguments) -> ()
fn eprintln(args: Arguments) -> ()
```

### 4.4 Network Module

**Module:** `std.net`

#### 4.4.1 TCP

```covibe
struct TcpListener:
    fn bind(addr: impl ToSocketAddrs) -> Result[TcpListener, IoError]
    fn accept(self: &Self) -> Result[(TcpStream, SocketAddr), IoError]
    fn local_addr(self: &Self) -> Result[SocketAddr, IoError]
    fn set_nonblocking(self: &Self, nonblocking: bool) -> Result[(), IoError]

struct TcpStream:
    fn connect(addr: impl ToSocketAddrs) -> Result[TcpStream, IoError]
    fn peer_addr(self: &Self) -> Result[SocketAddr, IoError]
    fn local_addr(self: &Self) -> Result[SocketAddr, IoError]
    fn shutdown(self: Self, how: Shutdown) -> Result[(), IoError]
    fn set_read_timeout(self: &Self, dur: Option[Duration]) -> Result[(), IoError]
    fn set_write_timeout(self: &Self, dur: Option[Duration]) -> Result[(), IoError]
    fn set_nodelay(self: &Self, nodelay: bool) -> Result[(), IoError]
    fn nodelay(self: &Self) -> Result[bool, IoError]
```

#### 4.4.2 UDP

```covibe
struct UdpSocket:
    fn bind(addr: impl ToSocketAddrs) -> Result[UdpSocket, IoError]
    fn send_to(self: &Self, buf: &[u8], addr: impl ToSocketAddrs) -> Result[usize, IoError]
    fn recv_from(self: &Self, buf: &mut [u8]) -> Result[(usize, SocketAddr), IoError]
    fn connect(self: &Self, addr: impl ToSocketAddrs) -> Result[(), IoError]
    fn send(self: &Self, buf: &[u8]) -> Result[usize, IoError]
    fn recv(self: &Self, buf: &mut [u8]) -> Result[usize, IoError]
    fn local_addr(self: &Self) -> Result[SocketAddr, IoError]
    fn broadcast(self: &Self) -> Result[bool, IoError]
    fn set_broadcast(self: &Self, broadcast: bool) -> Result[(), IoError]
```

#### 4.4.3 HTTP Client

```covibe
struct HttpClient:
    fn new() -> HttpClient
    fn get(url: &str) -> RequestBuilder
    fn post(url: &str) -> RequestBuilder
    fn put(url: &str) -> RequestBuilder
    fn delete(url: &str) -> RequestBuilder
    fn request(method: Method, url: &str) -> RequestBuilder

struct RequestBuilder:
    fn header(self, key: &str, value: &str) -> Self
    fn headers(self, headers: HeaderMap) -> Self
    fn body(self, body: impl Into[Body]) -> Self
    fn json[T](self, json: &T) -> Self where T: Serialize
    fn form[T](self, form: &T) -> Self where T: Serialize
    fn timeout(self, timeout: Duration) -> Self
    fn send(self) -> Result[Response, HttpError]

struct Response:
    fn status(self: &Self) -> StatusCode
    fn headers(self: &Self) -> &HeaderMap
    fn text(self) -> Result[String, HttpError]
    fn json[T](self) -> Result[T, HttpError] where T: Deserialize
    fn bytes(self) -> Result[Bytes, HttpError]
```

#### 4.4.4 HTTP Server

```covibe
struct HttpServer:
    fn new() -> HttpServer
    fn bind(self, addr: impl ToSocketAddrs) -> Result[Self, IoError]
    fn route(self, path: &str, handler: impl Handler) -> Self
    fn middleware(self, middleware: impl Middleware) -> Self
    fn run(self) -> Result[(), ServerError]

trait Handler:
    fn handle(self, req: Request) -> Result[Response, HandlerError]

struct Request:
    fn method(self: &Self) -> &Method
    fn uri(self: &Self) -> &Uri
    fn headers(self: &Self) -> &HeaderMap
    fn body(self) -> Body
    fn query[T](self: &Self) -> Result[T, ParseError] where T: Deserialize
    fn json[T](self) -> Result[T, ParseError] where T: Deserialize
```

### 4.5 Concurrency Module

**Module:** `std.concurrency`

```covibe
# Tasks
fn spawn[T](f: impl FnOnce() -> T + Send) -> JoinHandle[T]

struct JoinHandle[T]:
    fn join(self) -> Result[T, JoinError]
    fn is_finished(self: &Self) -> bool

# Channels
fn channel[T]() -> (Sender[T], Receiver[T])
fn bounded[T](capacity: usize) -> (Sender[T], Receiver[T])

struct Sender[T]:
    fn send(self: &Self, value: T) -> Result[(), SendError[T]]
    fn try_send(self: &Self, value: T) -> Result[(), TrySendError[T]]

struct Receiver[T]:
    fn recv(self: &Self) -> Result[T, RecvError]
    fn try_recv(self: &Self) -> Result[T, TryRecvError]
    fn recv_timeout(self: &Self, timeout: Duration) -> Result[T, RecvTimeoutError]

# Async
async fn async_function() -> T:
    # ...

fn await[T](future: impl Future[Output = T]) -> T

# Mutex and RwLock
struct Mutex[T]:
    fn new(value: T) -> Mutex[T]
    fn lock(self: &Self) -> MutexGuard[T]
    fn try_lock(self: &Self) -> Option[MutexGuard[T]]

struct RwLock[T]:
    fn new(value: T) -> RwLock[T]
    fn read(self: &Self) -> RwLockReadGuard[T]
    fn write(self: &Self) -> RwLockWriteGuard[T]
    fn try_read(self: &Self) -> Option[RwLockReadGuard[T]]
    fn try_write(self: &Self) -> Option[RwLockWriteGuard[T]]
```

### 4.6 Cryptography Module

**Module:** `std.crypto`

#### 4.6.1 Hashing

```covibe
# SHA-2
fn sha256(data: &[u8]) -> [u8; 32]
fn sha512(data: &[u8]) -> [u8; 64]

# SHA-3
fn sha3_256(data: &[u8]) -> [u8; 32]
fn sha3_512(data: &[u8]) -> [u8; 64]

# Blake3
fn blake3(data: &[u8]) -> [u8; 32]

struct Hasher:
    fn new() -> Hasher
    fn update(self: &mut Self, data: &[u8]) -> ()
    fn finalize(self) -> Hash
```

#### 4.6.2 Encryption

```covibe
# AES
struct Aes256Gcm:
    fn new(key: &[u8; 32]) -> Aes256Gcm
    fn encrypt(self: &Self, nonce: &[u8; 12], plaintext: &[u8]) -> Result[Vec[u8], CryptoError]
    fn decrypt(self: &Self, nonce: &[u8; 12], ciphertext: &[u8]) -> Result[Vec[u8], CryptoError]

# ChaCha20-Poly1305
struct ChaCha20Poly1305:
    fn new(key: &[u8; 32]) -> ChaCha20Poly1305
    fn encrypt(self: &Self, nonce: &[u8; 12], plaintext: &[u8]) -> Result[Vec[u8], CryptoError]
    fn decrypt(self: &Self, nonce: &[u8; 12], ciphertext: &[u8]) -> Result[Vec[u8], CryptoError]
```

#### 4.6.3 Key Derivation

```covibe
fn pbkdf2(password: &[u8], salt: &[u8], iterations: u32, output_len: usize) -> Vec[u8]
fn argon2id(password: &[u8], salt: &[u8], config: Argon2Config) -> Result[Vec[u8], CryptoError]
```

#### 4.6.4 Digital Signatures

```covibe
struct Ed25519KeyPair:
    fn generate() -> Ed25519KeyPair
    fn from_seed(seed: &[u8; 32]) -> Ed25519KeyPair
    fn public_key(self: &Self) -> &[u8; 32]
    fn sign(self: &Self, message: &[u8]) -> [u8; 64]
    fn verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool
```

### 4.7 Math Module

**Module:** `std.math`

```covibe
# Constants
const PI: f64
const E: f64
const TAU: f64

# Basic functions
fn abs[T](x: T) -> T where T: Num
fn min[T](a: T, b: T) -> T where T: Ord
fn max[T](a: T, b: T) -> T where T: Ord
fn sqrt(x: f64) -> f64
fn cbrt(x: f64) -> f64
fn pow(x: f64, y: f64) -> f64
fn exp(x: f64) -> f64
fn ln(x: f64) -> f64
fn log(x: f64, base: f64) -> f64
fn log10(x: f64) -> f64
fn log2(x: f64) -> f64

# Trigonometry
fn sin(x: f64) -> f64
fn cos(x: f64) -> f64
fn tan(x: f64) -> f64
fn asin(x: f64) -> f64
fn acos(x: f64) -> f64
fn atan(x: f64) -> f64
fn atan2(y: f64, x: f64) -> f64
fn sinh(x: f64) -> f64
fn cosh(x: f64) -> f64
fn tanh(x: f64) -> f64

# Rounding
fn floor(x: f64) -> f64
fn ceil(x: f64) -> f64
fn round(x: f64) -> f64
fn trunc(x: f64) -> f64

# Arbitrary precision
struct BigInt:
    fn from_i64(value: i64) -> BigInt
    fn from_str(s: &str) -> Result[BigInt, ParseError]
    fn to_string(self: &Self) -> String
    # Arithmetic operations via traits

struct BigRational:
    fn new(numerator: BigInt, denominator: BigInt) -> BigRational
    fn from_f64(value: f64) -> BigRational

# Complex numbers
struct Complex[T]:
    re: T
    im: T

    fn new(re: T, im: T) -> Complex[T]
    fn abs(self: &Self) -> T where T: Float
    fn arg(self: &Self) -> T where T: Float
```

### 4.8 Time Module

**Module:** `std.time`

```covibe
struct Duration:
    fn from_secs(secs: u64) -> Duration
    fn from_millis(millis: u64) -> Duration
    fn from_micros(micros: u64) -> Duration
    fn from_nanos(nanos: u64) -> Duration
    fn as_secs(self: &Self) -> u64
    fn as_millis(self: &Self) -> u128
    fn as_micros(self: &Self) -> u128
    fn as_nanos(self: &Self) -> u128

struct Instant:
    fn now() -> Instant
    fn elapsed(self: &Self) -> Duration
    fn duration_since(self: &Self, earlier: Instant) -> Duration

struct SystemTime:
    fn now() -> SystemTime
    fn duration_since(self: &Self, earlier: SystemTime) -> Result[Duration, SystemTimeError]
    fn elapsed(self: &Self) -> Result[Duration, SystemTimeError]

struct DateTime:
    fn now() -> DateTime
    fn from_timestamp(timestamp: i64) -> DateTime
    fn from_str(s: &str) -> Result[DateTime, ParseError]
    fn to_rfc3339(self: &Self) -> String
    fn year(self: &Self) -> i32
    fn month(self: &Self) -> u32
    fn day(self: &Self) -> u32
    fn hour(self: &Self) -> u32
    fn minute(self: &Self) -> u32
    fn second(self: &Self) -> u32
    fn timestamp(self: &Self) -> i64
```

### 4.9 Serialization Module

**Module:** `std.serde`

```covibe
# JSON
fn to_json[T](value: &T) -> Result[String, JsonError] where T: Serialize
fn from_json[T](s: &str) -> Result[T, JsonError] where T: Deserialize

struct JsonValue:
    fn parse(s: &str) -> Result[JsonValue, JsonError]
    fn to_string(self: &Self) -> String
    fn get(self: &Self, key: &str) -> Option[&JsonValue]
    fn as_str(self: &Self) -> Option[&str]
    fn as_i64(self: &Self) -> Option[i64]
    fn as_f64(self: &Self) -> Option[f64]
    fn as_bool(self: &Self) -> Option[bool>
    fn as_array(self: &Self) -> Option[&Vec[JsonValue]]
    fn as_object(self: &Self) -> Option[&Map[String, JsonValue]]

# TOML
fn to_toml[T](value: &T) -> Result[String, TomlError] where T: Serialize
fn from_toml[T](s: &str) -> Result[T, TomlError] where T: Deserialize

# YAML
fn to_yaml[T](value: &T) -> Result[String, YamlError] where T: Serialize
fn from_yaml[T](s: &str) -> Result[T, YamlError] where T: Deserialize

# MessagePack
fn to_msgpack[T](value: &T) -> Result[Vec[u8], MsgPackError] where T: Serialize
fn from_msgpack[T](bytes: &[u8]) -> Result[T, MsgPackError] where T: Deserialize
```

### 4.10 Testing Module

**Module:** `std.test`

```covibe
# Test attribute
#[test]
fn test_example():
    assert_eq(1 + 1, 2)

# Assertions
fn assert(condition: bool) -> ()
fn assert_eq[T](left: T, right: T) -> () where T: PartialEq + Debug
fn assert_ne[T](left: T, right: T) -> () where T: PartialEq + Debug

# Test configuration
#[test]
#[should_panic]
fn test_panics():
    panic("Expected panic")

#[test]
#[ignore]
fn expensive_test():
    # ...

# Benchmarking
#[bench]
fn bench_example(b: &mut Bencher):
    b.iter(|| {
        # Code to benchmark
    })
```

### 4.11 System Module

**Module:** `std.sys`

```covibe
# Process
fn args() -> Args
fn env() -> Env
fn current_dir() -> Result[PathBuf, IoError]
fn set_current_dir(path: impl AsRef[Path]) -> Result[(), IoError]
fn exit(code: i32) -> !

struct Command:
    fn new(program: impl AsRef[OsStr]) -> Command
    fn arg(self, arg: impl AsRef[OsStr]) -> Self
    fn args(self, args: impl IntoIterator[Item = impl AsRef[OsStr]]) -> Self
    fn env(self, key: impl AsRef[OsStr], val: impl AsRef[OsStr]) -> Self
    fn current_dir(self, dir: impl AsRef[Path]) -> Self
    fn stdin(self, cfg: Stdio) -> Self
    fn stdout(self, cfg: Stdio) -> Self
    fn stderr(self, cfg: Stdio) -> Self
    fn spawn(self) -> Result[Child, IoError]
    fn output(self) -> Result[Output, IoError]
    fn status(self) -> Result[ExitStatus, IoError]

# Path
struct Path:
    fn new(s: &str) -> &Path
    fn parent(self: &Self) -> Option[&Path]
    fn file_name(self: &Self) -> Option[&OsStr]
    fn extension(self: &Self) -> Option[&OsStr]
    fn exists(self: &Self) -> bool
    fn is_file(self: &Self) -> bool
    fn is_dir(self: &Self) -> bool
    fn join(self: &Self, path: impl AsRef[Path]) -> PathBuf

struct PathBuf:
    fn new() -> PathBuf
    fn push(self: &mut Self, path: impl AsRef[Path]) -> ()
    fn pop(self: &mut Self) -> bool
    fn set_file_name(self: &mut Self, file_name: impl AsRef[OsStr]) -> ()
    fn set_extension(self: &mut Self, extension: impl AsRef[OsStr]) -> bool
```

### 4.12 AI and Machine Learning Module

**Module:** `std.ml`

```covibe
# Tensor
struct Tensor[T]:
    fn zeros(shape: &[usize]) -> Tensor[T]
    fn ones(shape: &[usize]) -> Tensor[T]
    fn randn(shape: &[usize]) -> Tensor[T]
    fn from_vec(data: Vec[T], shape: &[usize]) -> Tensor[T]
    fn shape(self: &Self) -> &[usize]
    fn reshape(self, shape: &[usize]) -> Tensor[T]
    fn transpose(self) -> Tensor[T]
    fn matmul(self, other: &Tensor[T]) -> Tensor[T]
    fn add(self, other: &Tensor[T]) -> Tensor[T]
    fn sub(self, other: &Tensor[T]) -> Tensor[T]
    fn mul(self, other: &Tensor[T]) -> Tensor[T]
    fn div(self, other: &Tensor[T]) -> Tensor[T]
    fn sum(self) -> T
    fn mean(self) -> T
    fn backward(self) -> ()  # Compute gradients

# Automatic differentiation
fn enable_grad() -> ()
fn disable_grad() -> ()
fn no_grad[F, R](f: F) -> R where F: FnOnce() -> R

struct Variable[T]:
    fn new(tensor: Tensor[T], requires_grad: bool) -> Variable[T]
    fn backward(self: &Self) -> ()
    fn grad(self: &Self) -> Option[&Tensor[T]]
    fn zero_grad(self: &mut Self) -> ()
```

---

## Conclusion

This specification defines CoVibe's module system, error handling mechanisms, foreign function interface capabilities, and the complete standard library API. Together with the previous five parts, this forms a comprehensive foundation for the CoVibe programming language.

The module system provides a clean, hierarchical organization with explicit visibility controls and safe cyclic dependency resolution. Error handling combines the explicitness of `Result`/`Option` types with the convenience of `try/catch` sugar, all while maintaining compile-time type safety. The FFI support enables seamless interoperation with C, C++, Python, and JavaScript ecosystems. Finally, the standard library provides a comprehensive set of APIs covering common programming tasks from basic data structures to advanced features like cryptography, networking, and machine learning.

**Implementation Note**: The standard library signatures shown here represent the public API contract. The actual implementation will be developed in subsequent parts of this project, written in Rust for bootstrap purposes and gradually transitioned to self-hosted CoVibe code where performance permits.

---

**END OF PART 6 OF 100. Awaiting confirmation to proceed to Part 7.**
