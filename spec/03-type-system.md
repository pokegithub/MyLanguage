# CoVibe Language Specification
## Part 3: Type System Rules

**Version:** 1.0
**Date:** 2026-04-14
**Status:** Final

---

## Table of Contents

1. [Introduction](#introduction)
2. [Type Language](#type-language)
3. [Hindley-Milner Type Inference](#hindley-milner-type-inference)
4. [Algebraic Data Types](#algebraic-data-types)
5. [Generics and Trait Bounds](#generics-and-trait-bounds)
6. [Union and Intersection Types](#union-and-intersection-types)
7. [Refinement Types](#refinement-types)
8. [Dependent Type Subset](#dependent-type-subset)
9. [Effect Types](#effect-types)
10. [Linear Types](#linear-types)
11. [Variance](#variance)
12. [Type Definitions](#type-definitions)
13. [Special Types](#special-types)
14. [Numeric Tower](#numeric-tower)
15. [Subtyping Rules](#subtyping-rules)
16. [Type Equivalence](#type-equivalence)

---

## 1. Introduction

This document specifies the formal type system of the CoVibe programming language. CoVibe employs a rich, expressive type system that combines:

- **Hindley-Milner type inference** for automatic type deduction
- **Algebraic data types** for structured data modeling
- **Generics with trait bounds** for polymorphism
- **Advanced type features** including union types, refinement types, effect types, and linear types
- **Variance annotations** for safe covariance and contravariance

The type system is designed to provide strong static guarantees while minimizing explicit type annotations through sophisticated inference. All type checking occurs at compile time, ensuring zero runtime type overhead.

### 1.1 Design Principles

1. **Safety First**: The type system prevents common errors at compile time
2. **Inference Where Possible**: Types should be inferred when unambiguous
3. **Explicit When Necessary**: Ambiguous cases require explicit annotations
4. **Zero Cost Abstractions**: Type system features compile to efficient code
5. **Composability**: Type system features work together harmoniously

---

## 2. Type Language

### 2.1 Type Grammar

```ebnf
Type =
    | TypePath
    | TupleType
    | ArrayType
    | SliceType
    | FunctionType
    | ReferenceType
    | PointerType
    | UnionType
    | IntersectionType
    | RefinementType
    | EffectType
    | LinearType
    | ParenthesizedType
    | NeverType
    | TypeVariable

TypePath = Identifier ('::' Identifier)* TypeArgs?

TypeArgs = '<' Type (',' Type)* ','? '>'

TupleType = '(' (Type (',' Type)* ','?)? ')'

ArrayType = '[' Type ';' ConstExpr ']'

SliceType = '[' Type ']'

FunctionType = 'def' '(' (Type (',' Type)* ','?)? ')' ('->' Type)?

ReferenceType = '&' Lifetime? 'mut'? Type

PointerType = '*' ('const' | 'mut') Type

UnionType = Type '|' Type

IntersectionType = Type '&' Type

RefinementType = Type '{' Identifier ':' BoolExpr '}'

EffectType = Type '!' EffectSet

EffectSet = Identifier | '(' Identifier (',' Identifier)* ')'

LinearType = 'linear' Type

ParenthesizedType = '(' Type ')'

NeverType = '!'

TypeVariable = "'" Identifier
```

### 2.2 Type Constructor Kinds

CoVibe uses a simple kind system:

- `*` — The kind of all proper types (inhabited types)
- `* -> *` — The kind of type constructors (e.g., `Vec`, `Option`)
- `* -> * -> *` — The kind of binary type constructors (e.g., `Result`, `Map`)
- `Constraint` — The kind of trait constraints

Examples:
```covibe
int: *
Vec: * -> *
Result: * -> * -> *
Display: Constraint
```

---

## 3. Hindley-Milner Type Inference

### 3.1 Type Inference Algorithm

CoVibe uses Algorithm W (Hindley-Milner with extensions) for type inference. The algorithm performs:

1. **Constraint Generation**: Traverse the AST and generate type equality constraints
2. **Unification**: Solve constraints to find a most general unifier
3. **Generalization**: Introduce polymorphism at let bindings
4. **Instantiation**: Replace type schemes with fresh type variables at use sites

### 3.2 Type Schemes and Polymorphism

A type scheme has the form `∀α₁...αₙ. τ` where `α₁...αₙ` are type variables and `τ` is a type.

**Let-Polymorphism**: Type variables are generalized at let bindings:

```covibe
let id = lambda x: x
# id has type scheme: ∀α. α -> α

let y = id(42)      # α instantiated to int
let z = id("hello") # α instantiated to str
```

### 3.3 Value Restriction

To preserve soundness in the presence of mutable references, CoVibe applies the **value restriction**:

Only syntactic values are generalized. A syntactic value is:
- A lambda expression
- A constructor application (fully applied)
- A literal
- A variable

Non-values (function applications, mutable operations) are not generalized:

```covibe
# OK: id is a syntactic value (lambda)
let id = lambda x: x  # Type: ∀α. α -> α

# OK: pair is a syntactic value (constructor)
let pair = (1, 2)  # Type: (int, int)

# NOT generalized: compute is a function call
let compute = expensive_function()  # Type: T (monomorphic)

# OK with explicit type annotation
let compute: ∀α. α -> α = get_function()
```

### 3.4 Inference Rules

#### 3.4.1 Variable Rule

```
Γ ⊢ x : instantiate(Γ(x))
```

When a variable is used, instantiate its type scheme with fresh type variables.

#### 3.4.2 Abstraction Rule

```
Γ, x:α ⊢ e : τ
─────────────────
Γ ⊢ λx.e : α -> τ
```

Lambda expressions introduce function types.

#### 3.4.3 Application Rule

```
Γ ⊢ e₁ : τ₁ -> τ₂    Γ ⊢ e₂ : τ₁
───────────────────────────────────
Γ ⊢ e₁(e₂) : τ₂
```

Function application requires the argument type to match the parameter type.

#### 3.4.4 Let Rule (Generalization)

```
Γ ⊢ e₁ : τ₁    Γ, x:generalize(Γ, τ₁) ⊢ e₂ : τ₂
─────────────────────────────────────────────────
Γ ⊢ let x = e₁ in e₂ : τ₂
```

Let bindings generalize the type of the bound expression.

#### 3.4.5 Literal Rules

```
───────────
Γ ⊢ n : int

────────────────
Γ ⊢ f : float

──────────────
Γ ⊢ s : str

──────────────
Γ ⊢ b : bool
```

### 3.5 Unification

The unification algorithm `unify(τ₁, τ₂)` finds a substitution `θ` such that `θ(τ₁) = θ(τ₂)`:

**Unification Rules:**

1. `unify(α, τ) = [α ↦ τ]` if `α ∉ FV(τ)` (occurs check)
2. `unify(τ, α) = [α ↦ τ]` if `α ∉ FV(τ)`
3. `unify(T, T) = ∅` for type constructors T
4. `unify(τ₁ -> τ₂, τ₃ -> τ₄) = unify(τ₁, τ₃) ∘ unify(τ₂, τ₄)`
5. `unify(C<τ₁...τₙ>, C<σ₁...σₙ>) = unify(τ₁, σ₁) ∘ ... ∘ unify(τₙ, σₙ)`
6. Otherwise, fail with type error

**Occurs Check**: Prevents infinite types by ensuring a type variable doesn't occur in the type it's being unified with.

### 3.6 Generalization and Instantiation

**Generalization** `generalize(Γ, τ)`:
```
generalize(Γ, τ) = ∀α₁...αₙ. τ
where {α₁...αₙ} = FV(τ) \ FV(Γ)
```

**Instantiation** `instantiate(∀α₁...αₙ. τ)`:
```
instantiate(∀α₁...αₙ. τ) = [α₁ ↦ β₁, ..., αₙ ↦ βₙ]τ
where β₁...βₙ are fresh type variables
```

### 3.7 Type Inference Examples

```covibe
# Example 1: Simple inference
let double = lambda x: x + x
# Inferred type: int -> int (from + operator)

# Example 2: Polymorphic identity
let id = lambda x: x
# Inferred type: ∀α. α -> α

# Example 3: Composition
let compose = lambda f, g, x: f(g(x))
# Inferred type: ∀α, β, γ. (β -> γ) -> (α -> β) -> α -> γ

# Example 4: Type constraints from usage
let process = lambda items: items.map(lambda x: x + 1)
# Inferred type: ∀α: Add<int>. [α] -> [α]
```

---

## 4. Algebraic Data Types

### 4.1 Product Types (Structs)

Product types combine multiple values:

```covibe
struct Point:
    x: int
    y: int

struct Generic<T>:
    value: T
    count: int
```

**Type Rule for Struct Construction:**
```
Γ ⊢ e₁ : τ₁    ...    Γ ⊢ eₙ : τₙ
─────────────────────────────────────────
Γ ⊢ S { f₁: e₁, ..., fₙ: eₙ } : S
where struct S has fields f₁: τ₁, ..., fₙ: τₙ
```

**Type Rule for Field Access:**
```
Γ ⊢ e : S    S has field f: τ
──────────────────────────────
Γ ⊢ e.f : τ
```

### 4.2 Sum Types (Enums)

Sum types represent a choice between alternatives:

```covibe
enum Option<T>:
    Some(T)
    None

enum Result<T, E>:
    Ok(T)
    Err(E)

enum Message:
    Quit
    Move:
        x: int
        y: int
    Write(str)
    ChangeColor(int, int, int)
```

**Type Rule for Enum Construction:**
```
Γ ⊢ e : τ
─────────────────────────────
Γ ⊢ E::Variant(e) : E<...>
where enum E<...> has variant Variant(τ)
```

### 4.3 Recursive Types

CoVibe supports recursive algebraic data types:

```covibe
enum List<T>:
    Cons(T, Box<List<T>>)
    Nil

enum Tree<T>:
    Leaf(T)
    Node(Box<Tree<T>>, Box<Tree<T>>)
```

**Positivity Requirement**: Type variables in recursive types must appear in positive positions only (not as function parameters in contravariant positions).

Valid:
```covibe
enum Valid<T>:
    A(T)
    B(Box<Valid<T>>)  # T in positive position
```

Invalid:
```covibe
enum Invalid<T>:
    A(def(T) -> int)  # T in negative position - REJECTED
```

---

## 5. Generics and Trait Bounds

### 5.1 Generic Type Parameters

Functions, structs, enums, and traits can be generic over types:

```covibe
def identity<T>(x: T) -> T:
    return x

struct Container<T>:
    value: T

enum Either<L, R>:
    Left(L)
    Right(R)
```

### 5.2 Trait Bounds

Type parameters can be constrained by traits:

```covibe
def print_all<T: Display>(items: [T]):
    for item in items:
        print(item)

def compare<T: Ord>(a: T, b: T) -> bool:
    return a < b
```

**Syntax:**
```ebnf
TypeBound = Trait ('+' Trait)*

Trait = TypePath TypeArgs?
```

### 5.3 Where Clauses

Complex bounds can be expressed with where clauses:

```covibe
def complex<T, U>(t: T, u: U) -> T
where
    T: Clone + Display,
    U: Into<T>:
    return t.clone()

def multi_bound<T, U, V>(t: T, u: U, v: V)
where
    T: Display + Debug,
    U: Into<T> + Clone,
    V: From<U>:
    # ...
```

### 5.4 Associated Types

Traits can have associated types:

```covibe
trait Iterator:
    type Item

    def next(&mut self) -> Option<Self::Item>

trait Graph:
    type Node
    type Edge

    def neighbors(&self, node: Self::Node) -> [Self::Node]
```

**Type Rule for Associated Types:**
```
Γ ⊢ e : T    T: Trait    Trait has associated type A
──────────────────────────────────────────────────────
Γ ⊢ <T as Trait>::A : *
```

### 5.5 Higher-Ranked Types

CoVibe supports higher-ranked polymorphism for function types:

```covibe
# for<'a> means: for all lifetimes 'a
type Callback = for<'a> def(&'a str) -> &'a str

def apply_callback(f: for<'a> def(&'a str) -> &'a str, s: &str) -> &str:
    return f(s)
```

---

## 6. Union and Intersection Types

### 6.1 Union Types

Union types represent a value that can be one of several types:

```covibe
type IntOrString = int | str

def process(value: int | str | bool):
    match value:
        case x: int => print(f"Integer: {x}")
        case s: str => print(f"String: {s}")
        case b: bool => print(f"Boolean: {b}")
```

**Subtyping for Unions:**
```
τ <: τ₁ | τ₂    (if τ <: τ₁ or τ <: τ₂)
τ₁ | τ₂ <: τ    (if τ₁ <: τ and τ₂ <: τ)
```

**Type Rule for Union:**
```
Γ ⊢ e : τ    τ <: τ₁ | τ₂
──────────────────────────
Γ ⊢ e : τ₁ | τ₂
```

### 6.2 Intersection Types

Intersection types represent a value that satisfies multiple type constraints:

```covibe
trait Drawable:
    def draw(&self)

trait Clickable:
    def click(&self)

def interact(obj: Drawable & Clickable):
    obj.draw()
    obj.click()
```

**Subtyping for Intersections:**
```
τ₁ & τ₂ <: τ₁
τ₁ & τ₂ <: τ₂
τ <: τ₁ & τ₂    (if τ <: τ₁ and τ <: τ₂)
```

### 6.3 Union and Intersection Properties

**Distributivity:**
```
(τ₁ | τ₂) & τ₃ ≡ (τ₁ & τ₃) | (τ₂ & τ₃)
```

**Absorption:**
```
τ | (τ & σ) ≡ τ
τ & (τ | σ) ≡ τ
```

**De Morgan's Laws** (for subtyping):
```
¬(τ₁ | τ₂) ≡ ¬τ₁ & ¬τ₂
¬(τ₁ & τ₂) ≡ ¬τ₁ | ¬τ₂
```

---

## 7. Refinement Types

### 7.1 Refinement Type Syntax

Refinement types allow attaching logical predicates to types:

```covibe
type Nat = int { x: x >= 0 }
type NonEmpty<T> = [T] { xs: xs.len() > 0 }
type Even = int { n: n % 2 == 0 }

def sqrt(x: int { x >= 0 }) -> float:
    # x is guaranteed to be non-negative
    return x.sqrt()

def head<T>(list: [T] { list.len() > 0 }) -> T:
    # list is guaranteed to be non-empty
    return list[0]
```

### 7.2 Refinement Type Grammar

```ebnf
RefinementType = Type '{' Identifier ':' Predicate '}'

Predicate =
    | Identifier
    | Literal
    | Predicate BinaryOp Predicate
    | UnaryOp Predicate
    | Identifier '.' Identifier
    | Identifier '(' Arguments ')'
    | '(' Predicate ')'

BinaryOp = '==' | '!=' | '<' | '<=' | '>' | '>=' | '&&' | '||'

UnaryOp = '!' | '-'
```

### 7.3 Refinement Type Checking

**Subtyping Rule:**
```
Γ ⊢ τ₁ <: τ₂    Γ, x:τ₁ ⊢ P₁ ⇒ P₂
────────────────────────────────────
Γ ⊢ {x:τ₁ | P₁} <: {x:τ₂ | P₂}
```

Where `P₁ ⇒ P₂` means P₁ implies P₂ (checked via SMT solver).

### 7.4 Refinement Type Examples

```covibe
# Division with non-zero check
def divide(a: int, b: int { b != 0 }) -> int:
    return a / b  # Safe: b is non-zero

# Array indexing with bounds check
def get<T>(arr: [T], i: int { i >= 0 && i < arr.len() }) -> T:
    return arr[i]  # Safe: i is within bounds

# Positive number squaring preserves positivity
def square(x: int { x > 0 }) -> int { result: result > 0 }:
    return x * x  # Compiler verifies result > 0
```

---

## 8. Dependent Type Subset

### 8.1 Value-Dependent Types

CoVibe supports a limited form of dependent types where types can depend on compile-time constant values:

```covibe
# Array length is part of the type
def first<T, const N: usize>(arr: [T; N]) -> T
where
    N > 0:
    return arr[0]

# Vector type depends on dimension
struct Vec<const N: usize>:
    data: [float; N]

def dot<const N: usize>(a: Vec<N>, b: Vec<N>) -> float:
    var sum = 0.0
    for i in 0..N:
        sum += a.data[i] * b.data[i]
    return sum
```

### 8.2 Const Generics

Const generic parameters are compile-time constants that can be used in types:

```covibe
struct Matrix<T, const ROWS: usize, const COLS: usize>:
    data: [[T; COLS]; ROWS]

impl<T, const ROWS: usize, const COLS: usize> Matrix<T, ROWS, COLS>:
    def new(value: T) -> Self
    where
        T: Clone:
        return Matrix {
            data: [[value.clone(); COLS]; ROWS]
        }

# Type-level arithmetic
def transpose<T, const R: usize, const C: usize>(
    m: Matrix<T, R, C>
) -> Matrix<T, C, R>:
    # ...
```

### 8.3 Type Families

Type families are compile-time functions from types/values to types:

```covibe
type family ArrayOf<T, const N: usize> = [T; N]

type family Result<T>:
    type Output = Result<T, Error>

# Associated type families
trait Collection:
    type family Element
    type family Iterator

impl Collection for Vec<T>:
    type Element = T
    type Iterator = VecIter<T>
```

---

## 9. Effect Types

### 9.1 Effect System Overview

CoVibe's effect system tracks computational effects at the type level:

```covibe
# Pure function: no effects
def add(a: int, b: int) -> int:
    return a + b

# IO effect
def read_file(path: str) -> str!IO:
    # Performs I/O
    return File::read(path)

# Multiple effects
def process() -> int!(IO, Async):
    let data = await fetch_data()  # Async effect
    let file = read_file("config")  # IO effect
    return parse(data)
```

### 9.2 Effect Types

**Effect Annotations:**
```ebnf
EffectType = Type '!' EffectSet

EffectSet =
    | Identifier
    | '(' Identifier (',' Identifier)* ')'

BuiltinEffect =
    | 'Pure'    # No side effects
    | 'IO'      # Input/output
    | 'Async'   # Asynchronous computation
    | 'Unsafe'  # Unsafe operations
    | 'Diverge' # May not terminate
    | 'Panic'   # May panic
```

### 9.3 Effect Inference

Effects are inferred from function bodies:

```covibe
# Automatically inferred as !IO
def log(msg: str):
    print(msg)  # print has effect IO

# Automatically inferred as !(IO, Async)
async def fetch_and_log(url: str):
    let data = await http.get(url)  # Async
    print(data)                      # IO
```

### 9.4 Effect Subtyping

**Effect Subsumption:**
```
Pure <: E for all effects E
E₁ <: E₁ | E₂
E₂ <: E₁ | E₂
```

**Function Effect Subtyping:**
```
τ₁ -> τ₂!E₁ <: τ₁ -> τ₂!E₂    if E₁ <: E₂
```

### 9.5 Effect Handlers

Effects can be handled to transform effect types:

```covibe
def handle_io<T>(f: def() -> T!IO) -> T:
    with io_context():
        return f()

def handle_async<T>(f: def() -> T!Async) -> T:
    return runtime.block_on(f())
```

---

## 10. Linear Types

### 10.1 Linear Type Semantics

Linear types ensure resources are used exactly once:

```covibe
linear struct FileHandle:
    fd: int

linear struct SocketConnection:
    socket: int

# Linear types must be consumed exactly once
def use_file(file: linear FileHandle):
    # file must be consumed before function returns
    file.close()  # Consumes file
```

### 10.2 Linear Type Rules

**Use-Exactly-Once Rule:**
```
Every linear value must be used exactly once:
- Not used: compile error (resource leak)
- Used multiple times: compile error (double use)
```

**Linearity Checking:**
```
Γ ⊢ e : linear τ
────────────────────────
Γ \ {linear variables} ⊢ e consumed
```

### 10.3 Splitting Linear Types

Linear types can be split into components:

```covibe
linear struct Pair<A, B>:
    first: A
    second: B

def use_pair(p: linear Pair<int, str>):
    let (a, b) = p.split()  # Consumes p, produces a and b
    use_int(a)
    use_str(b)
```

### 10.4 Linear Functions

Functions can require linear arguments:

```covibe
# Takes linear argument, returns linear result
def transform(x: linear Resource) -> linear Resource:
    # Must consume x and return a new linear value
    let y = x.process()
    return y

# Linear closure
let handler: linear def() -> () = move || {
    use_resource()
}
handler()  # Consumes the closure
```

### 10.5 Linear Type Examples

```covibe
linear struct Transaction:
    db: DatabaseConnection

def execute(tx: linear Transaction) -> Result<(), Error>:
    match tx.commit():
        case Ok(()) => Ok(())
        case Err(e) =>
            tx.rollback()  # OK: tx consumed on error path
            Err(e)

# Compile error: tx not consumed
def bad(tx: linear Transaction):
    if should_commit():
        tx.commit()
    # ERROR: tx not consumed on else branch
```

---

## 11. Variance

### 11.1 Variance Definitions

Variance describes how subtyping of type parameters affects subtyping of generic types:

- **Covariant** (`+T`): If `A <: B`, then `C<A> <: C<B>`
- **Contravariant** (`-T`): If `A <: B`, then `C<B> <: C<A>`
- **Invariant** (no annotation): No subtyping relationship

### 11.2 Variance Annotations

```covibe
# Covariant
struct Producer<+T>:
    value: T

# Contravariant
struct Consumer<-T>:
    handler: def(T) -> ()

# Invariant
struct Cell<T>:
    value: mut T
```

### 11.3 Variance Rules

**Position-Based Variance:**

1. **Covariant positions:**
   - Return types
   - Immutable fields
   - Array element types (immutable)

2. **Contravariant positions:**
   - Function parameter types

3. **Invariant positions:**
   - Mutable fields
   - Both parameter and return positions

### 11.4 Variance Inference

Variance is inferred from usage:

```covibe
# Inferred as covariant because T only appears in covariant positions
struct Box<T>:
    value: T

# Inferred as contravariant because T only appears in parameters
struct Processor<T>:
    process: def(T) -> int

# Inferred as invariant because T appears in mutable field
struct RefCell<T>:
    value: mut T
```

### 11.5 Variance Examples

```covibe
# Covariant example
struct Animal:
    pass

struct Dog:
    pass

impl Animal for Dog:
    pass

let dogs: Producer<Dog> = Producer { value: Dog() }
let animals: Producer<Animal> = dogs  # OK: Producer is covariant

# Contravariant example
let animal_consumer: Consumer<Animal> = Consumer {
    handler: |a| print(a)
}
let dog_consumer: Consumer<Dog> = animal_consumer  # OK: Consumer is contravariant
```

---

## 12. Type Definitions

### 12.1 Type Aliases

Type aliases create alternative names for existing types:

```covibe
type Int32 = i32
type StringMap<T> = Map<str, T>
type Result<T> = Result<T, Error>

# Generic type alias
type Callback<T, R> = def(T) -> R

# Alias with bounds
type Comparable = int | float | str
```

**Type Rule:**
```
type Alias = τ
────────────────
Alias ≡ τ
```

### 12.2 Newtype Pattern

Newtypes create distinct types with the same representation:

```covibe
newtype UserId = int
newtype Email = str

# UserId and int are not interchangeable
let user_id: UserId = UserId(42)
let n: int = 42

# ERROR: Type mismatch
# let x: UserId = n

# OK: Explicit conversion
let y: UserId = UserId(n)
```

### 12.3 Opaque Types

Opaque types hide implementation details:

```covibe
opaque type Handle = int

def create_handle() -> Handle:
    return Handle::from_int(generate_id())

# Outside this module, Handle's representation is hidden
# Can only be created and manipulated through defined functions
```

### 12.4 Type Families (Type-Level Functions)

Type families compute types from other types:

```covibe
type family ElementOf<C>:
    case [T]: T
    case Vec<T>: T
    case Map<K, V>: V
    case Set<T>: T

def first<C>(container: C) -> ElementOf<C>:
    # Return type determined by container type
    return container.get(0)
```

---

## 13. Special Types

### 13.1 Never Type (`!`)

The never type represents computations that never return normally:

```covibe
def panic(msg: str) -> !:
    # Never returns
    abort(msg)

def diverge() -> !:
    loop:
        continue

def match_all(x: Option<int>) -> int:
    match x:
        case Some(n) => n
        case None => panic("unexpected None")  # ! coerces to int
```

**Type Rules:**
```
Γ ⊢ e : !
─────────────  (Never-Elim)
Γ ⊢ e : τ

! <: τ for all τ
```

### 13.2 Unit Type (`()`)

The unit type has exactly one value, `()`:

```covibe
def print_message(msg: str) -> ():
    print(msg)  # Returns ()

let unit_value: () = ()
```

**Type Rules:**
```
────────────
Γ ⊢ () : ()
```

### 13.3 Bottom Type (⊥)

The bottom type is the subtype of all types (theoretical, not directly expressable):

```
⊥ <: τ for all τ
```

Used internally for type checking unreachable code.

### 13.4 Top Type

CoVibe has no explicit top type, but `Any` trait serves a similar purpose:

```covibe
trait Any:
    # All types implicitly implement Any
    pass

def accept_anything(x: impl Any):
    # x can be any type
    pass
```

---

## 14. Numeric Tower

### 14.1 Numeric Type Hierarchy

CoVibe defines a numeric tower with implicit conversions:

```
                Integer
                   |
        +----------+----------+
        |                     |
    Signed Int          Unsigned Int
        |                     |
    +---+---+             +---+---+
    |   |   |             |   |   |
   i8 i16 i32 i64 i128   u8 u16 u32 u64 u128
   isize                 usize

                Float
                  |
            +-----+-----+
            |           |
           f32         f64
```

### 14.2 Numeric Types

**Integer Types:**
- `i8`, `i16`, `i32`, `i64`, `i128`, `isize` — Signed integers
- `u8`, `u16`, `u32`, `u64`, `u128`, `usize` — Unsigned integers
- `int` — Alias for `i64` (default integer type)

**Floating-Point Types:**
- `f32` — 32-bit IEEE 754 floating point
- `f64` — 64-bit IEEE 754 floating point
- `float` — Alias for `f64` (default floating point type)

### 14.3 Numeric Traits

```covibe
trait Num:
    # Basic numeric operations
    def add(self, other: Self) -> Self
    def sub(self, other: Self) -> Self
    def mul(self, other: Self) -> Self
    def div(self, other: Self) -> Self

trait Integer: Num:
    # Integer-specific operations
    def rem(self, other: Self) -> Self
    def div_euclid(self, other: Self) -> Self

trait Float: Num:
    # Floating-point specific operations
    def sqrt(self) -> Self
    def sin(self) -> Self
    def cos(self) -> Self
```

### 14.4 Numeric Conversions

**Explicit Conversions:**
```covibe
let x: i32 = 42
let y: i64 = x as i64
let z: f64 = x as f64
```

**Trait-Based Conversions:**
```covibe
trait Into<T>:
    def into(self) -> T

trait From<T>:
    def from(value: T) -> Self

# Automatic implementations
impl From<i32> for i64:
    def from(x: i32) -> i64:
        return x as i64

let a: i32 = 42
let b: i64 = i64::from(a)
let c: i64 = a.into()
```

### 14.5 Numeric Literal Inference

Integer literals are typed as `int` by default, but can be inferred from context:

```covibe
let a = 42        # Type: int (i64)
let b: i32 = 42   # Type: i32
let c = 42i32     # Type: i32
let d = 42u64     # Type: u64

let e = 3.14      # Type: float (f64)
let f: f32 = 3.14 # Type: f32
let g = 3.14f32   # Type: f32
```

---

## 15. Subtyping Rules

### 15.1 Subtyping Relation

CoVibe uses structural subtyping for certain types:

**Reflexivity:**
```
τ <: τ
```

**Transitivity:**
```
τ₁ <: τ₂    τ₂ <: τ₃
────────────────────
τ₁ <: τ₃
```

### 15.2 Function Subtyping

Functions are contravariant in parameters and covariant in return types:

```
σ₁ <: τ₁    τ₂ <: σ₂
────────────────────────
(τ₁ -> τ₂) <: (σ₁ -> σ₂)
```

### 15.3 Tuple Subtyping

Tuples are covariant in all positions:

```
τ₁ <: σ₁    ...    τₙ <: σₙ
────────────────────────────
(τ₁, ..., τₙ) <: (σ₁, ..., σₙ)
```

### 15.4 Reference Subtyping

**Lifetime Subtyping:**
```
'a outlives 'b  (written 'a: 'b)
────────────────────────────────
&'a T <: &'b T
```

**Mutability:**
```
&T <: &mut T    (NOT valid - immutable is not subtype of mutable)
&mut T <: &T    (NOT valid - mutable is not subtype of immutable)
```

### 15.5 Trait Object Subtyping

```
Trait₁ :> Trait₂    (Trait₂ extends Trait₁)
────────────────────────────────────────────
dyn Trait₂ <: dyn Trait₁
```

---

## 16. Type Equivalence

### 16.1 Nominal Equivalence

Structs and enums use nominal equivalence (name-based):

```covibe
struct A:
    x: int

struct B:
    x: int

# A and B are NOT equivalent, even though they have the same structure
```

### 16.2 Structural Equivalence

Certain types use structural equivalence:

**Tuples:**
```covibe
(int, str) ≡ (int, str)
```

**Functions:**
```covibe
def(int) -> str ≡ def(int) -> str
```

**Arrays:**
```covibe
[int; 10] ≡ [int; 10]
```

### 16.3 Type Alias Transparency

Type aliases are transparent and equivalent to their definitions:

```covibe
type UserId = int

# UserId ≡ int for all purposes
```

### 16.4 Phantom Type Parameters

Type parameters that don't appear in the type's definition:

```covibe
struct PhantomData<T>:
    # T not used in fields
    pass

# PhantomData<int> ≢ PhantomData<str>
# Even though they have the same structure
```

---

## Conclusion

This specification defines the complete type system of the CoVibe programming language. The type system combines:

- **Hindley-Milner inference** for automatic type deduction with let-polymorphism
- **Algebraic data types** for expressive data modeling
- **Generics and traits** for polymorphism and abstraction
- **Advanced features** including union types, refinement types, effect types, and linear types
- **Variance annotations** for safe covariance and contravariance
- **Comprehensive numeric tower** with well-defined conversions

The type system provides strong static guarantees while maintaining inference where possible, minimizing the burden of explicit type annotations. All type checking is performed at compile time, ensuring zero runtime overhead for type safety.

This type system serves as the foundation for CoVibe's semantic analysis phases, which will be specified in subsequent parts of this document series.

---

**Document History:**
- 2026-04-14: Initial version 1.0 — Complete type system specification
