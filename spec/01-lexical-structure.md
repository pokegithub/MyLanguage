# CoVibe Language Specification
## Part 1: Lexical Structure and Core Syntax Grammar

**Version:** 1.0
**Date:** 2026-04-14
**Status:** Approved

---

## Table of Contents

1. [Introduction](#introduction)
2. [Lexical Elements](#lexical-elements)
3. [Character Set and Encoding](#character-set-and-encoding)
4. [Whitespace and Line Terminators](#whitespace-and-line-terminators)
5. [Indentation Semantics](#indentation-semantics)
6. [Comments](#comments)
7. [Identifiers](#identifiers)
8. [Keywords](#keywords)
9. [Operators and Punctuation](#operators-and-punctuation)
10. [Literals](#literals)
11. [Operator Precedence and Associativity](#operator-precedence-and-associativity)
12. [Expression Grammar](#expression-grammar)
13. [Statement Grammar](#statement-grammar)

---

## 1. Introduction

This document formally defines the lexical structure and core syntax grammar of the CoVibe programming language. CoVibe combines the performance of C, the safety of Rust, and the clarity of Python into a modern, expressive systems programming language.

The grammar is specified using Extended Backus-Naur Form (EBNF) notation with the following conventions:

- `Terminal` — literal text in code
- `NonTerminal` — grammar rule reference
- `x | y` — alternation (x or y)
- `x?` — optional (zero or one occurrence)
- `x*` — repetition (zero or more occurrences)
- `x+` — repetition (one or more occurrences)
- `(x y)` — grouping
- `[a-z]` — character range
- `/* comment */` — EBNF comment

---

## 2. Lexical Elements

A CoVibe source file is processed as a sequence of Unicode characters encoded in UTF-8. The lexical analysis phase transforms this character stream into a sequence of tokens. Each token belongs to one of the following categories:

- **Keywords** — reserved identifiers with special meaning
- **Identifiers** — names for variables, functions, types, etc.
- **Literals** — constant values (numbers, strings, booleans)
- **Operators** — symbols denoting operations
- **Punctuation** — structural delimiters
- **Comments** — documentation and annotations (ignored by parser)
- **Whitespace** — spaces, tabs, newlines (structurally significant for indentation)

---

## 3. Character Set and Encoding

### 3.1 Source Encoding

All CoVibe source files must be encoded in **UTF-8** without a byte order mark (BOM). A source file that begins with the UTF-8 encoded Unicode BOM character (U+FEFF) must have that character stripped before lexical analysis.

### 3.2 Character Classes

```ebnf
UnicodeChar = /* any valid Unicode code point */
Letter      = [A-Za-z]
Digit       = [0-9]
BinaryDigit = [01]
OctalDigit  = [0-7]
HexDigit    = [0-9A-Fa-f]
```

### 3.3 Unicode Categories

CoVibe recognizes the following Unicode character categories for identifier formation:

- **XID_Start** — characters valid at the start of an identifier (Unicode property)
- **XID_Continue** — characters valid in the body of an identifier (Unicode property)

---

## 4. Whitespace and Line Terminators

### 4.1 Whitespace Characters

```ebnf
Whitespace = Space | Tab | FormFeed

Space     = U+0020  /* ASCII space */
Tab       = U+0009  /* ASCII horizontal tab */
FormFeed  = U+000C  /* ASCII form feed */
```

Whitespace characters are used to separate tokens but are otherwise ignored except:
1. When determining indentation levels (see Section 5)
2. Inside string literals
3. When preceded by a backslash (line continuation)

### 4.2 Line Terminators

```ebnf
LineTerminator =
    | U+000A        /* LF - Line Feed */
    | U+000D        /* CR - Carriage Return */
    | U+000D U+000A /* CRLF - CR followed by LF */
```

Line terminators separate logical lines. A CRLF sequence is treated as a single line terminator.

### 4.3 Line Continuation

A backslash immediately followed by a line terminator causes the line break to be ignored:

```ebnf
LineContinuation = '\' LineTerminator
```

Example:
```covibe
let x = 1 + 2 + \
        3 + 4
```

---

## 5. Indentation Semantics

CoVibe uses **significant indentation** to define block structure, similar to Python but with stricter rules.

### 5.1 Indentation Rules

1. **Consistency**: A source file must use either spaces or tabs for indentation, but not both. Mixing spaces and tabs is a lexical error.

2. **Indentation Unit**:
   - If spaces are used, the indentation unit is **4 spaces** (recommended).
   - If tabs are used, the indentation unit is **1 tab**.

3. **INDENT Token**: When a line's indentation increases from the previous logical line, an `INDENT` token is generated.

4. **DEDENT Token**: When a line's indentation decreases, one or more `DEDENT` tokens are generated to match the indentation level.

5. **Indentation Stack**: The lexer maintains a stack of indentation levels. The first line of the file is at indentation level 0.

### 5.2 Indentation EBNF

```ebnf
IndentationChar = Space | Tab
Indentation     = IndentationChar*
```

### 5.3 Indentation Algorithm

1. At the start of each logical line (after line continuation resolution), count the indentation.
2. If indentation increases compared to the top of the stack:
   - Push the new level onto the stack.
   - Emit an `INDENT` token.
3. If indentation decreases:
   - Pop levels from the stack until matching indentation is found.
   - Emit one `DEDENT` token for each level popped.
   - If no matching level exists, this is an indentation error.
4. If indentation matches the top of the stack, no token is emitted.

### 5.4 Special Cases

- **Blank lines**: Lines containing only whitespace and comments are ignored for indentation purposes.
- **Inside delimiters**: Indentation rules do not apply inside `()`, `[]`, or `{}`.
- **Explicit braces**: When `{` opens a block, indentation inside is ignored until the matching `}`.

### 5.5 Example

```covibe
def factorial(n: int) -> int:
    if n <= 1:
        return 1
    else:
        return n * factorial(n - 1)
```

Token sequence (simplified):
```
DEF IDENT("factorial") LPAREN IDENT("n") COLON IDENT("int") RPAREN
ARROW IDENT("int") COLON NEWLINE INDENT IF IDENT("n") LE INT(1) COLON
NEWLINE INDENT RETURN INT(1) NEWLINE DEDENT ELSE COLON NEWLINE INDENT
RETURN IDENT("n") STAR IDENT("factorial") LPAREN IDENT("n") MINUS INT(1)
RPAREN NEWLINE DEDENT DEDENT
```

---

## 6. Comments

CoVibe supports three types of comments:

### 6.1 Line Comments

Line comments begin with `#` and extend to the end of the line:

```ebnf
LineComment = '#' (~LineTerminator)* LineTerminator?
```

Example:
```covibe
# This is a line comment
let x = 42  # inline comment
```

### 6.2 Block Comments

Block comments begin with `/*` and end with `*/`. They may span multiple lines and can be nested:

```ebnf
BlockComment = '/*' (BlockComment | ~'*/')* '*/'
```

Example:
```covibe
/*
 * This is a block comment.
 * It can span multiple lines.
 */

/* Nested /* comments */ are allowed */
```

### 6.3 Documentation Comments

Documentation comments are special comments used for automatic documentation generation:

- **Line doc comments**: Begin with `##` or `#!`
- **Block doc comments**: Begin with `/**` or `/*!`

```ebnf
DocLineComment  = '##' (~LineTerminator)* LineTerminator?
ModuleDocComment = '#!' (~LineTerminator)* LineTerminator?
DocBlockComment = '/**' (~'*/')* '*/'
ModuleBlockDoc  = '/*!' (~'*/')* '*/'
```

Example:
```covibe
## This function computes the factorial of n.
##
## # Arguments
## * `n` - A non-negative integer
##
## # Returns
## The factorial of n
def factorial(n: int) -> int:
    if n <= 1:
        return 1
    else:
        return n * factorial(n - 1)
```

---

## 7. Identifiers

### 7.1 Identifier Syntax

Identifiers are names used for variables, functions, types, modules, and other entities.

```ebnf
Identifier = IdentifierStart IdentifierContinue*

IdentifierStart    = Letter | '_' | UnicodeXIDStart
IdentifierContinue = Letter | Digit | '_' | UnicodeXIDContinue

UnicodeXIDStart    = /* Unicode XID_Start property */
UnicodeXIDContinue = /* Unicode XID_Continue property */
```

### 7.2 Identifier Rules

1. Identifiers are **case-sensitive**: `foo`, `Foo`, and `FOO` are distinct.
2. Identifiers cannot be keywords unless prefixed with `@` (raw identifier escape).
3. Leading underscores have conventional meaning:
   - `_name` — private/internal (convention)
   - `__name` — name mangling (for class members)
   - `_` — wildcard/discard pattern

### 7.3 Raw Identifiers

To use a keyword as an identifier, prefix it with `@`:

```ebnf
RawIdentifier = '@' Keyword
```

Example:
```covibe
let @if = 42  # Use keyword "if" as identifier
```

### 7.4 Unicode Identifiers

CoVibe fully supports Unicode identifiers following Unicode Standard Annex #31:

```covibe
let π = 3.14159
let 変数 = "variable"
let переменная = "variable"
```

### 7.5 Reserved Identifiers

The following identifiers are reserved for future use and cannot be used even with raw identifier syntax:

- `__builtin`
- `__compiler`
- `__covibe`

---

## 8. Keywords

Keywords are reserved identifiers with special syntactic meaning. They cannot be used as regular identifiers.

### 8.1 Keyword List

```ebnf
Keyword =
    /* Declaration keywords */
    | 'def' | 'let' | 'var' | 'const'
    | 'struct' | 'enum' | 'trait' | 'impl' | 'type'
    | 'class' | 'interface'

    /* Control flow keywords */
    | 'if' | 'elif' | 'else' | 'match' | 'case'
    | 'for' | 'while' | 'loop' | 'break' | 'continue' | 'return'
    | 'yield' | 'await'

    /* Type keywords */
    | 'int' | 'float' | 'bool' | 'str' | 'char'
    | 'i8' | 'i16' | 'i32' | 'i64' | 'i128' | 'isize'
    | 'u8' | 'u16' | 'u32' | 'u64' | 'u128' | 'usize'
    | 'f32' | 'f64'

    /* Module and visibility keywords */
    | 'import' | 'from' | 'as' | 'export'
    | 'pub' | 'priv' | 'protected'

    /* Memory and ownership keywords */
    | 'ref' | 'mut' | 'move' | 'copy' | 'clone'
    | 'box' | 'alloc' | 'defer' | 'drop'
    | 'static' | 'unsafe'

    /* Concurrency keywords */
    | 'async' | 'spawn' | 'send' | 'recv' | 'select'

    /* Boolean and special literals */
    | 'true' | 'false' | 'none' | 'null'

    /* Operator keywords */
    | 'and' | 'or' | 'not' | 'in' | 'is'

    /* Other keywords */
    | 'self' | 'Self' | 'super'
    | 'where' | 'with' | 'try' | 'catch' | 'finally'
    | 'raise' | 'assert'
    | 'lambda' | 'comptime'
    | 'macro' | 'extern'
```

### 8.2 Contextual Keywords

The following identifiers have special meaning only in specific contexts and can be used as regular identifiers elsewhere:

- `get`, `set` (in property declarations)
- `operator` (in operator overloading)
- `constructor`, `destructor` (in class definitions)
- `abstract`, `virtual`, `override` (in inheritance contexts)
- `effect`, `pure` (in effect system)
- `linear` (in linear type contexts)

---

## 9. Operators and Punctuation

### 9.1 Arithmetic Operators

```ebnf
ArithmeticOp =
    | '+'   /* addition, unary plus */
    | '-'   /* subtraction, unary minus */
    | '*'   /* multiplication */
    | '/'   /* division */
    | '//'  /* integer division */
    | '%'   /* modulo */
    | '**'  /* exponentiation */
```

### 9.2 Comparison Operators

```ebnf
ComparisonOp =
    | '=='  /* equality */
    | '!='  /* inequality */
    | '<'   /* less than */
    | '<='  /* less than or equal */
    | '>'   /* greater than */
    | '>='  /* greater than or equal */
    | '<=>' /* three-way comparison (spaceship) */
```

### 9.3 Logical Operators

```ebnf
LogicalOp =
    | 'and' | '&&'  /* logical AND */
    | 'or'  | '||'  /* logical OR */
    | 'not' | '!'   /* logical NOT */
```

### 9.4 Bitwise Operators

```ebnf
BitwiseOp =
    | '&'   /* bitwise AND */
    | '|'   /* bitwise OR */
    | '^'   /* bitwise XOR */
    | '~'   /* bitwise NOT */
    | '<<'  /* left shift */
    | '>>'  /* right shift */
    | '>>>' /* unsigned right shift */
```

### 9.5 Assignment Operators

```ebnf
AssignmentOp =
    | '='    /* simple assignment */
    | '+='   /* add and assign */
    | '-='   /* subtract and assign */
    | '*='   /* multiply and assign */
    | '/='   /* divide and assign */
    | '//='  /* integer divide and assign */
    | '%='   /* modulo and assign */
    | '**='  /* exponentiate and assign */
    | '&='   /* bitwise AND and assign */
    | '|='   /* bitwise OR and assign */
    | '^='   /* bitwise XOR and assign */
    | '<<='  /* left shift and assign */
    | '>>='  /* right shift and assign */
    | '>>>=' /* unsigned right shift and assign */
```

### 9.6 Other Operators

```ebnf
OtherOp =
    | '->'   /* function return type arrow */
    | '=>'   /* match arm arrow */
    | '..'   /* range (exclusive end) */
    | '..='  /* range (inclusive end) */
    | '...'  /* variadic, spread */
    | '?'    /* optional chaining, error propagation */
    | '??'   /* null coalescing */
    | '?:'   /* ternary operator (with condition) */
    | '::'   /* path separator, scope resolution */
    | '@'    /* decorator, raw identifier prefix */
    | '$'    /* macro variable prefix */
    | '|>'   /* pipe operator */
    | '<|'   /* reverse pipe operator */
```

### 9.7 Punctuation

```ebnf
Punctuation =
    | '('  | ')'   /* parentheses */
    | '['  | ']'   /* square brackets */
    | '{'  | '}'   /* curly braces */
    | ','           /* comma */
    | '.'           /* dot, member access */
    | ':'           /* colon */
    | ';'           /* semicolon */
    | '#'           /* hash (for comments and directives) */
```

---

## 10. Literals

### 10.1 Integer Literals

Integer literals represent constant integer values in various bases.

```ebnf
IntegerLiteral =
    | DecimalLiteral
    | BinaryLiteral
    | OctalLiteral
    | HexadecimalLiteral

DecimalLiteral     = Digit (Digit | '_')* IntegerSuffix?
BinaryLiteral      = '0' [bB] BinaryDigit (BinaryDigit | '_')* IntegerSuffix?
OctalLiteral       = '0' [oO] OctalDigit (OctalDigit | '_')* IntegerSuffix?
HexadecimalLiteral = '0' [xX] HexDigit (HexDigit | '_')* IntegerSuffix?

IntegerSuffix =
    | 'i8' | 'i16' | 'i32' | 'i64' | 'i128' | 'isize'
    | 'u8' | 'u16' | 'u32' | 'u64' | 'u128' | 'usize'
```

Examples:
```covibe
42          # decimal
1_000_000   # decimal with separators
0b1010      # binary
0o755       # octal
0xDEADBEEF  # hexadecimal
42i64       # 64-bit signed integer
255u8       # 8-bit unsigned integer
```

### 10.2 Floating-Point Literals

Floating-point literals represent constant real numbers.

```ebnf
FloatLiteral =
    | Digit+ '.' Digit+ Exponent? FloatSuffix?
    | Digit+ Exponent FloatSuffix?
    | '.' Digit+ Exponent? FloatSuffix?

Exponent    = [eE] [+-]? Digit+
FloatSuffix = 'f32' | 'f64'
```

Examples:
```covibe
3.14
2.5e10
2.5e-3
1e6
.5
3.14f32
2.0f64
```

### 10.3 Boolean Literals

```ebnf
BooleanLiteral = 'true' | 'false'
```

### 10.4 Character Literals

Character literals represent a single Unicode character.

```ebnf
CharLiteral = '\'' (CharContent | EscapeSequence) '\''

CharContent = ~['\\\r\n\t]

EscapeSequence =
    | '\\' [nrt\\'"0]           /* simple escapes */
    | '\\x' HexDigit HexDigit   /* byte escape */
    | '\\u{' HexDigit+ '}'      /* Unicode escape */
```

Escape sequences:
- `\n` — newline (U+000A)
- `\r` — carriage return (U+000D)
- `\t` — horizontal tab (U+0009)
- `\\` — backslash
- `\'` — single quote
- `\"` — double quote
- `\0` — null character (U+0000)
- `\xHH` — byte escape (2 hex digits)
- `\u{HHHHHH}` — Unicode escape (1-6 hex digits)

Examples:
```covibe
'a'
'π'
'\n'
'\u{1F600}'  # emoji
```

### 10.5 String Literals

CoVibe supports multiple string literal forms.

#### 10.5.1 Plain String Literals

```ebnf
StringLiteral = '"' StringContent* '"'

StringContent =
    | StringChar
    | EscapeSequence
    | LineContinuation

StringChar = ~["\\\r\n]
```

Example:
```covibe
"Hello, world!"
"Line 1\nLine 2"
"Unicode: \u{1F680}"
```

#### 10.5.2 Raw String Literals

Raw strings do not interpret escape sequences. They begin with `r` followed by zero or more hash symbols `#`, then a quote. They end with a quote followed by the same number of hash symbols.

```ebnf
RawStringLiteral = 'r' '#'* '"' ~["]* '"' '#'*
```

Examples:
```covibe
r"C:\Users\name\path"
r#"String with "quotes" inside"#
r##"String with "# inside"##
```

#### 10.5.3 Format String Literals (f-strings)

Format strings allow embedded expressions using `{}` syntax.

```ebnf
FormatStringLiteral = 'f' '"' FStringContent* '"'

FStringContent =
    | StringChar
    | EscapeSequence
    | Interpolation

Interpolation = '{' Expression FormatSpec? '}'
FormatSpec    = ':' ~[}]+
```

Examples:
```covibe
f"Hello, {name}!"
f"Result: {x + y}"
f"Formatted: {value:06.2f}"
```

#### 10.5.4 Heredoc String Literals

Heredoc strings are multi-line string literals delimited by triple quotes.

```ebnf
HeredocLiteral = '"""' HeredocContent* '"""'
HeredocContent = /* any character including newlines */
```

Example:
```covibe
let text = """
    This is a multi-line string.
    It preserves indentation and newlines.
    No escape sequences needed for quotes: " ' "
"""
```

#### 10.5.5 Byte String Literals

Byte strings represent sequences of bytes rather than Unicode text.

```ebnf
ByteStringLiteral = 'b' '"' ByteContent* '"'
ByteContent       = /* ASCII characters and byte escapes */
```

Example:
```covibe
b"ASCII only"
b"\x00\x01\x02\xFF"
```

### 10.6 None Literal

The `none` keyword represents the absence of a value (similar to null/nil in other languages).

```ebnf
NoneLiteral = 'none'
```

---

## 11. Operator Precedence and Associativity

Operators are listed from highest to lowest precedence. Operators on the same line have equal precedence.

| Precedence | Operator | Description | Associativity |
|------------|----------|-------------|---------------|
| 18 | `::` | Path separator | Left |
| 17 | `()` `[]` `.` | Call, subscript, member access | Left |
| 16 | `?` | Optional chaining | Left |
| 15 | `**` | Exponentiation | Right |
| 14 | `+` `-` `!` `~` `not` `&` `*` | Unary plus, minus, logical NOT, bitwise NOT, reference, dereference | Right |
| 13 | `as` | Type cast | Left |
| 12 | `*` `/` `//` `%` | Multiplication, division, integer division, modulo | Left |
| 11 | `+` `-` | Addition, subtraction | Left |
| 10 | `<<` `>>` `>>>` | Bit shifts | Left |
| 9 | `&` | Bitwise AND | Left |
| 8 | `^` | Bitwise XOR | Left |
| 7 | `\|` | Bitwise OR | Left |
| 6 | `..` `..=` | Range operators | Left |
| 5 | `==` `!=` `<` `<=` `>` `>=` `<=>` `is` `in` | Comparison operators | Left |
| 4 | `and` `&&` | Logical AND | Left |
| 3 | `or` `\|\|` | Logical OR | Left |
| 2 | `??` | Null coalescing | Right |
| 1 | `?:` | Ternary conditional | Right |
| 0 | `=` `+=` `-=` `*=` `/=` etc. `\|>` `<\|` | Assignment, pipe operators | Right |

**Notes:**
- Operators with higher precedence bind more tightly.
- Left associativity: `a + b + c` parses as `(a + b) + c`
- Right associativity: `a ** b ** c` parses as `a ** (b ** c)`
- Comparison operators can be chained: `a < b < c` means `(a < b) and (b < c)`

---

## 12. Expression Grammar

Expressions represent computations that produce values.

### 12.1 Primary Expressions

```ebnf
Expression = AssignmentExpression

PrimaryExpression =
    | Identifier
    | Literal
    | 'self'
    | 'Self'
    | 'super'
    | ParenthesizedExpression
    | ArrayExpression
    | TupleExpression
    | StructExpression
    | LambdaExpression
    | IfExpression
    | MatchExpression
    | BlockExpression

Literal =
    | IntegerLiteral
    | FloatLiteral
    | BooleanLiteral
    | CharLiteral
    | StringLiteral
    | NoneLiteral

ParenthesizedExpression = '(' Expression ')'

ArrayExpression = '[' (Expression (',' Expression)* ','?)? ']'

TupleExpression = '(' Expression (',' Expression)+ ','? ')'

StructExpression =
    | TypePath '{' (FieldInit (',' FieldInit)* ','?)? '}'

FieldInit = Identifier (':' Expression)?
```

### 12.2 Postfix Expressions

```ebnf
PostfixExpression =
    | PrimaryExpression
    | PostfixExpression '(' Arguments? ')'       /* function call */
    | PostfixExpression '[' Expression ']'       /* subscript */
    | PostfixExpression '.' Identifier           /* member access */
    | PostfixExpression '?'                      /* error propagation */
    | PostfixExpression '.' IntegerLiteral       /* tuple field access */
    | PostfixExpression '::' PathSegment         /* qualified path */

Arguments = Expression (',' Expression)* ','?

PathSegment = Identifier GenericArgs?

GenericArgs = '<' (TypeExpression (',' TypeExpression)* ','?)? '>'
```

### 12.3 Unary Expressions

```ebnf
UnaryExpression =
    | PostfixExpression
    | '+' UnaryExpression      /* unary plus */
    | '-' UnaryExpression      /* unary minus */
    | '!' UnaryExpression      /* logical NOT */
    | 'not' UnaryExpression    /* logical NOT (keyword) */
    | '~' UnaryExpression      /* bitwise NOT */
    | '&' 'mut'? UnaryExpression /* borrow (mutable or immutable) */
    | '*' UnaryExpression      /* dereference */
    | 'box' UnaryExpression    /* heap allocation */
    | 'move' UnaryExpression   /* explicit move */
    | 'copy' UnaryExpression   /* explicit copy */
```

### 12.4 Binary Expressions

```ebnf
BinaryExpression =
    | UnaryExpression
    | BinaryExpression '**' BinaryExpression
    | BinaryExpression ('*' | '/' | '//' | '%') BinaryExpression
    | BinaryExpression ('+' | '-') BinaryExpression
    | BinaryExpression ('<<' | '>>' | '>>>') BinaryExpression
    | BinaryExpression '&' BinaryExpression
    | BinaryExpression '^' BinaryExpression
    | BinaryExpression '|' BinaryExpression
    | BinaryExpression ('..' | '..=') BinaryExpression
    | BinaryExpression ('==' | '!=' | '<' | '<=' | '>' | '>=' | '<=>' | 'is' | 'in') BinaryExpression
    | BinaryExpression ('and' | '&&') BinaryExpression
    | BinaryExpression ('or' | '||') BinaryExpression
    | BinaryExpression '??' BinaryExpression
    | BinaryExpression 'as' TypeExpression
```

### 12.5 Ternary Expression

```ebnf
TernaryExpression =
    | BinaryExpression
    | BinaryExpression '?' Expression ':' TernaryExpression
```

### 12.6 Pipe Expression

```ebnf
PipeExpression =
    | TernaryExpression
    | PipeExpression '|>' PipeTarget
    | PipeTarget '<|' PipeExpression

PipeTarget = PostfixExpression
```

### 12.7 Assignment Expression

```ebnf
AssignmentExpression =
    | PipeExpression
    | UnaryExpression AssignmentOp AssignmentExpression

AssignmentOp =
    | '=' | '+=' | '-=' | '*=' | '/=' | '//=' | '%=' | '**='
    | '&=' | '|=' | '^=' | '<<=' | '>>=' | '>>>='
```

### 12.8 Lambda Expression

```ebnf
LambdaExpression =
    | 'lambda' Parameters ('->' TypeExpression)? ':' Expression
    | '|' Parameters '|' ('->' TypeExpression)? Expression

Parameters = Parameter (',' Parameter)* ','?

Parameter = Pattern (':' TypeExpression)?
```

### 12.9 If Expression

```ebnf
IfExpression =
    | 'if' Expression ':' BlockOrExpression
      ('elif' Expression ':' BlockOrExpression)*
      ('else' ':' BlockOrExpression)?

BlockOrExpression = Block | Expression
```

### 12.10 Match Expression

```ebnf
MatchExpression =
    | 'match' Expression ':'?
      INDENT
      MatchArm+
      DEDENT

MatchArm =
    | 'case' Pattern Guard? '=>' Expression

Guard = 'if' Expression
```

### 12.11 Block Expression

```ebnf
BlockExpression =
    | '{' Statement* Expression? '}'
    | INDENT Statement* Expression? DEDENT
```

### 12.12 List Comprehension

```ebnf
ListComprehension =
    '[' Expression ComprehensionClause+ ']'

ComprehensionClause =
    | 'for' Pattern 'in' Expression
    | 'if' Expression
```

---

## 13. Statement Grammar

Statements perform actions but do not produce values (or produce the unit type `()`).

### 13.1 Statement Types

```ebnf
Statement =
    | LetStatement
    | AssignmentStatement
    | ExpressionStatement
    | ControlFlowStatement
    | ImportStatement
    | DeclarationStatement
    | EmptyStatement

EmptyStatement = NEWLINE
```

### 13.2 Let Statement

```ebnf
LetStatement =
    | ('let' | 'var' | 'const') Pattern (':' TypeExpression)? ('=' Expression)? NEWLINE
```

**Semantics:**
- `let` — immutable binding (default)
- `var` — mutable binding
- `const` — compile-time constant

Examples:
```covibe
let x = 42
var y: int = 10
const PI = 3.14159
let (a, b) = (1, 2)
```

### 13.3 Assignment Statement

```ebnf
AssignmentStatement = AssignmentExpression NEWLINE
```

Example:
```covibe
x = 10
y += 5
```

### 13.4 Expression Statement

```ebnf
ExpressionStatement = Expression NEWLINE
```

An expression followed by a newline is treated as a statement.

### 13.5 Control Flow Statements

```ebnf
ControlFlowStatement =
    | IfStatement
    | WhileStatement
    | ForStatement
    | LoopStatement
    | MatchStatement
    | BreakStatement
    | ContinueStatement
    | ReturnStatement
    | YieldStatement
    | RaiseStatement
    | TryStatement
    | DeferStatement

IfStatement =
    | 'if' Expression ':' Block
      ('elif' Expression ':' Block)*
      ('else' ':' Block)?

WhileStatement =
    | 'while' Expression ':' Block

ForStatement =
    | 'for' Pattern 'in' Expression ':' Block

LoopStatement =
    | 'loop' ':' Block

MatchStatement =
    | 'match' Expression ':'
      INDENT
      MatchArm+
      DEDENT

BreakStatement = 'break' Expression? NEWLINE

ContinueStatement = 'continue' NEWLINE

ReturnStatement = 'return' (Expression (',' Expression)*)? NEWLINE

YieldStatement = 'yield' Expression NEWLINE

RaiseStatement = 'raise' Expression NEWLINE

TryStatement =
    | 'try' ':' Block
      ('catch' Pattern ':' Block)+
      ('finally' ':' Block)?

DeferStatement = 'defer' Expression NEWLINE

Block =
    | INDENT Statement+ DEDENT
    | '{' Statement* '}'
```

### 13.6 Import Statement

```ebnf
ImportStatement =
    | 'import' ImportPath ('as' Identifier)? NEWLINE
    | 'from' ImportPath 'import' ImportList NEWLINE
    | 'from' ImportPath 'import' '*' NEWLINE

ImportPath = Identifier ('.' Identifier)*

ImportList = ImportItem (',' ImportItem)* ','?

ImportItem = Identifier ('as' Identifier)?
```

Examples:
```covibe
import math
import collections.HashMap as HashMap
from std.io import File, read, write
from std.net import *
```

### 13.7 Declaration Statements

```ebnf
DeclarationStatement =
    | FunctionDeclaration
    | StructDeclaration
    | EnumDeclaration
    | TraitDeclaration
    | ImplDeclaration
    | TypeAliasDeclaration
    | ExternDeclaration

FunctionDeclaration =
    | Decorator* Visibility? 'async'? 'def' Identifier
      GenericParams? '(' FunctionParams? ')' ('->' TypeExpression)?
      (':' Block | '=' Expression NEWLINE)

Decorator = '@' PostfixExpression NEWLINE

Visibility = 'pub' | 'priv' | 'protected'

GenericParams = '<' GenericParam (',' GenericParam)* ','? '>'

GenericParam = Identifier (':' TypeBound)?

TypeBound = TypeExpression ('+' TypeExpression)*

FunctionParams = FunctionParam (',' FunctionParam)* ','?

FunctionParam =
    | Pattern ':' TypeExpression ('=' Expression)?
    | '...' Identifier  /* variadic */

StructDeclaration =
    | Visibility? 'struct' Identifier GenericParams? ':'
      INDENT StructField+ DEDENT
    | Visibility? 'struct' Identifier GenericParams?
      '(' TupleStructFields? ')' NEWLINE

StructField = Visibility? Identifier ':' TypeExpression NEWLINE

TupleStructFields = TypeExpression (',' TypeExpression)* ','?

EnumDeclaration =
    | Visibility? 'enum' Identifier GenericParams? ':'
      INDENT EnumVariant+ DEDENT

EnumVariant =
    | Identifier NEWLINE
    | Identifier '(' TupleStructFields? ')' NEWLINE
    | Identifier ':' INDENT StructField+ DEDENT

TraitDeclaration =
    | Visibility? 'trait' Identifier GenericParams? (':' TypeBound)? ':'
      INDENT TraitItem+ DEDENT

TraitItem =
    | FunctionSignature
    | TypeAssociation

FunctionSignature =
    | 'def' Identifier GenericParams? '(' FunctionParams? ')'
      ('->' TypeExpression)? NEWLINE

TypeAssociation =
    | 'type' Identifier (':' TypeBound)? NEWLINE

ImplDeclaration =
    | 'impl' GenericParams? TypeExpression ('for' TypeExpression)? ':'
      INDENT (FunctionDeclaration | TypeAliasDeclaration)+ DEDENT

TypeAliasDeclaration =
    | Visibility? 'type' Identifier GenericParams? '=' TypeExpression NEWLINE

ExternDeclaration =
    | 'extern' StringLiteral? ':'
      INDENT ExternItem+ DEDENT

ExternItem =
    | FunctionSignature
    | 'static' Identifier ':' TypeExpression NEWLINE
```

---

## 13.8 Type Expression Grammar

Type expressions describe the types of values.

```ebnf
TypeExpression =
    | TypePath
    | TupleType
    | ArrayType
    | FunctionType
    | ReferenceType
    | PointerType
    | UnionType
    | IntersectionType
    | OptionalType
    | ResultType
    | ParenthesizedType

TypePath = Identifier ('::' Identifier)* GenericArgs?

TupleType = '(' (TypeExpression (',' TypeExpression)* ','?)? ')'

ArrayType = '[' TypeExpression (';' Expression)? ']'

FunctionType = 'def' '(' (TypeExpression (',' TypeExpression)* ','?)? ')'
               ('->' TypeExpression)?

ReferenceType = '&' 'mut'? TypeExpression

PointerType = '*' ('const' | 'mut') TypeExpression

UnionType = TypeExpression '|' TypeExpression

IntersectionType = TypeExpression '&' TypeExpression

OptionalType = TypeExpression '?'

ResultType = TypeExpression '!'

ParenthesizedType = '(' TypeExpression ')'
```

---

## 13.9 Pattern Grammar

Patterns are used in match expressions, let bindings, and function parameters.

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

LiteralPattern = Literal

IdentifierPattern = 'mut'? Identifier ('@' Pattern)?

WildcardPattern = '_'

TuplePattern = '(' (Pattern (',' Pattern)* ','?)? ')'

StructPattern =
    | TypePath '{' (FieldPattern (',' FieldPattern)* ','? (',' '..')?)? '}'

FieldPattern = Identifier (':' Pattern)?

EnumPattern = TypePath ('(' (Pattern (',' Pattern)* ','?)? ')')?

OrPattern = Pattern ('|' Pattern)+

RangePattern = Literal ('..' | '..=') Literal

ReferencePattern = '&' 'mut'? Pattern

BoxPattern = 'box' Pattern
```

---

## Conclusion

This specification defines the complete lexical structure and core syntax grammar of the CoVibe programming language. It covers all token types, indentation rules, comment syntax, identifier rules, operators, literals (including plain strings, f-strings, heredocs, and raw strings), operator precedence, expression grammar, and statement grammar in formal EBNF notation.

This document serves as the normative reference for lexical analysis and parsing in CoVibe compiler implementations and provides a foundation for the subsequent specification parts covering control flow, type system, memory model, concurrency, and other advanced features.

---

**Document History:**
- 2026-04-14: Initial version 1.0 — Complete lexical structure and core syntax grammar
