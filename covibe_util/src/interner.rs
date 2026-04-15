//! String interning for efficient symbol storage.
//!
//! The interner maintains a global pool of unique strings (symbols) and
//! assigns each a unique integer identifier. This allows for fast equality
//! comparison and reduced memory usage for frequently-used identifiers.

use std::fmt;
use std::sync::Arc;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;

/// A unique identifier for an interned string.
///
/// Symbols can be compared for equality in O(1) time and are small (4 bytes).
/// The actual string content can be retrieved from the Interner.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(u32);

impl Symbol {
    /// Creates a Symbol from a raw u32. Used internally by the interner.
    const fn from_raw(raw: u32) -> Self {
        Symbol(raw)
    }

    /// Returns the raw u32 value.
    pub const fn as_raw(self) -> u32 {
        self.0
    }

    /// A sentinel value for an invalid or placeholder symbol.
    pub const INVALID: Symbol = Symbol(u32::MAX);
}

impl fmt::Debug for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Symbol({})", self.0)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// Thread-safe string interner.
///
/// The interner stores strings in an arena and maintains a bidirectional
/// mapping between strings and their Symbol IDs. Once a string is interned,
/// it remains valid for the lifetime of the Interner.
#[derive(Debug, Clone)]
pub struct Interner {
    inner: Arc<RwLock<InternerInner>>,
}

#[derive(Debug)]
struct InternerInner {
    /// Map from string content to Symbol ID
    string_to_symbol: FxHashMap<Arc<str>, Symbol>,

    /// Map from Symbol ID to string content
    symbol_to_string: Vec<Arc<str>>,
}

impl Default for InternerInner {
    fn default() -> Self {
        let mut inner = InternerInner {
            string_to_symbol: FxHashMap::default(),
            symbol_to_string: Vec::new(),
        };

        // Pre-intern common keywords and symbols
        for keyword in COMMON_KEYWORDS {
            inner.intern_fresh(keyword);
        }

        inner
    }
}

impl InternerInner {
    /// Interns a string that is known to not already exist.
    fn intern_fresh(&mut self, s: &str) -> Symbol {
        let symbol = Symbol::from_raw(self.symbol_to_string.len() as u32);
        let arc_str: Arc<str> = Arc::from(s);

        self.symbol_to_string.push(arc_str.clone());
        self.string_to_symbol.insert(arc_str, symbol);

        symbol
    }
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

impl Interner {
    /// Creates a new Interner with common keywords pre-interned.
    pub fn new() -> Self {
        Interner {
            inner: Arc::new(RwLock::new(InternerInner::default())),
        }
    }

    /// Interns a string and returns its Symbol.
    ///
    /// If the string has already been interned, returns the existing Symbol.
    /// Otherwise, creates a new Symbol for this string.
    pub fn intern(&self, s: &str) -> Symbol {
        // Fast path: check if already interned (read lock only)
        {
            let inner = self.inner.read();
            if let Some(&symbol) = inner.string_to_symbol.get(s) {
                return symbol;
            }
        }

        // Slow path: intern the string (write lock required)
        let mut inner = self.inner.write();

        // Double-check after acquiring write lock (another thread may have interned it)
        if let Some(&symbol) = inner.string_to_symbol.get(s) {
            return symbol;
        }

        inner.intern_fresh(s)
    }

    /// Retrieves the string content for a Symbol.
    ///
    /// Returns None if the symbol is invalid or unknown.
    pub fn resolve(&self, symbol: Symbol) -> Option<Arc<str>> {
        let inner = self.inner.read();
        inner.symbol_to_string.get(symbol.0 as usize).cloned()
    }

    /// Retrieves the string content for a Symbol as a string slice.
    ///
    /// Returns an empty string if the symbol is invalid.
    pub fn resolve_str(&self, symbol: Symbol) -> String {
        self.resolve(symbol)
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    /// Returns the number of unique strings interned.
    pub fn len(&self) -> usize {
        let inner = self.inner.read();
        inner.symbol_to_string.len()
    }

    /// Returns true if no strings have been interned.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Pre-interns a batch of strings for efficiency.
    pub fn intern_batch<I>(&self, strings: I)
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let mut inner = self.inner.write();

        for s in strings {
            let s = s.as_ref();
            if !inner.string_to_symbol.contains_key(s) {
                inner.intern_fresh(s);
            }
        }
    }
}

/// Common keywords that are pre-interned for efficiency.
const COMMON_KEYWORDS: &[&str] = &[
    // Keywords
    "fn", "let", "mut", "const", "if", "else", "elif", "for", "while",
    "loop", "break", "continue", "return", "match", "case", "in", "as",
    "type", "struct", "enum", "trait", "impl", "where", "pub", "use",
    "mod", "extern", "unsafe", "async", "await", "spawn", "select",
    "defer", "comptime", "macro", "import", "export",

    // Primitive types
    "int", "i8", "i16", "i32", "i64", "i128",
    "uint", "u8", "u16", "u32", "u64", "u128",
    "float", "f32", "f64", "bool", "char", "str", "string",
    "void", "never",

    // Literals
    "true", "false", "null", "none", "some",

    // Special identifiers
    "self", "Self", "super", "main",
];

/// Symbols for commonly-used keywords and identifiers.
///
/// These constants provide O(1) access to frequently-used symbols without
/// needing to call intern().
#[derive(Debug)]
#[allow(non_snake_case)]
pub struct KnownSymbols {
    // Keywords
    pub kw_fn: Symbol,
    pub kw_let: Symbol,
    pub kw_mut: Symbol,
    pub kw_const: Symbol,
    pub kw_if: Symbol,
    pub kw_else: Symbol,
    pub kw_for: Symbol,
    pub kw_while: Symbol,
    pub kw_return: Symbol,
    pub kw_match: Symbol,
    pub kw_struct: Symbol,
    pub kw_enum: Symbol,
    pub kw_trait: Symbol,
    pub kw_impl: Symbol,
    pub kw_true: Symbol,
    pub kw_false: Symbol,
    pub kw_self: Symbol,
    pub kw_Self: Symbol,
}

impl KnownSymbols {
    /// Initializes the known symbols from an interner.
    pub fn new(interner: &Interner) -> Self {
        KnownSymbols {
            kw_fn: interner.intern("fn"),
            kw_let: interner.intern("let"),
            kw_mut: interner.intern("mut"),
            kw_const: interner.intern("const"),
            kw_if: interner.intern("if"),
            kw_else: interner.intern("else"),
            kw_for: interner.intern("for"),
            kw_while: interner.intern("while"),
            kw_return: interner.intern("return"),
            kw_match: interner.intern("match"),
            kw_struct: interner.intern("struct"),
            kw_enum: interner.intern("enum"),
            kw_trait: interner.intern("trait"),
            kw_impl: interner.intern("impl"),
            kw_true: interner.intern("true"),
            kw_false: interner.intern("false"),
            kw_self: interner.intern("self"),
            kw_Self: interner.intern("Self"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_basic() {
        let interner = Interner::new();

        let sym1 = interner.intern("hello");
        let sym2 = interner.intern("world");
        let sym3 = interner.intern("hello"); // Should return same symbol as sym1

        assert_eq!(sym1, sym3);
        assert_ne!(sym1, sym2);
    }

    #[test]
    fn test_resolve() {
        let interner = Interner::new();

        let sym = interner.intern("test_string");
        let resolved = interner.resolve(sym).unwrap();

        assert_eq!(resolved.as_ref(), "test_string");
    }

    #[test]
    fn test_resolve_str() {
        let interner = Interner::new();

        let sym = interner.intern("foo");
        assert_eq!(interner.resolve_str(sym), "foo");

        // Invalid symbol should return empty string
        assert_eq!(interner.resolve_str(Symbol::INVALID), "");
    }

    #[test]
    fn test_preinterned_keywords() {
        let interner = Interner::new();

        // Keywords should already be interned
        let fn_sym1 = interner.intern("fn");
        let fn_sym2 = interner.intern("fn");

        assert_eq!(fn_sym1, fn_sym2);
        assert_eq!(interner.resolve_str(fn_sym1), "fn");
    }

    #[test]
    fn test_known_symbols() {
        let interner = Interner::new();
        let known = KnownSymbols::new(&interner);

        assert_eq!(interner.resolve_str(known.kw_fn), "fn");
        assert_eq!(interner.resolve_str(known.kw_let), "let");
        assert_eq!(interner.resolve_str(known.kw_true), "true");
        assert_eq!(interner.resolve_str(known.kw_self), "self");
    }

    #[test]
    fn test_intern_batch() {
        let interner = Interner::new();

        let strings = vec!["alpha", "beta", "gamma"];
        interner.intern_batch(strings.clone());

        // All strings should be interned
        for s in strings {
            let sym = interner.intern(s);
            assert_eq!(interner.resolve_str(sym), s);
        }
    }

    #[test]
    fn test_interner_len() {
        let interner = Interner::new();

        // Some keywords are pre-interned
        let initial_len = interner.len();
        assert!(initial_len > 0);

        interner.intern("new_symbol");
        assert_eq!(interner.len(), initial_len + 1);

        // Interning the same symbol again shouldn't increase length
        interner.intern("new_symbol");
        assert_eq!(interner.len(), initial_len + 1);
    }

    #[test]
    fn test_unicode_strings() {
        let interner = Interner::new();

        let sym1 = interner.intern("hello_世界");
        let sym2 = interner.intern("🚀");

        assert_eq!(interner.resolve_str(sym1), "hello_世界");
        assert_eq!(interner.resolve_str(sym2), "🚀");
    }

    #[test]
    fn test_empty_string() {
        let interner = Interner::new();

        let sym = interner.intern("");
        assert_eq!(interner.resolve_str(sym), "");
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let interner = Interner::new();
        let interner_clone = interner.clone();

        let handle = thread::spawn(move || {
            interner_clone.intern("thread_test")
        });

        let sym1 = interner.intern("thread_test");
        let sym2 = handle.join().unwrap();

        assert_eq!(sym1, sym2);
    }
}
