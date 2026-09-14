//! The miniature expression grammar the scrubber constant-folds conditions with.
//!
//! **Role:** lexes a condition, parses it, and folds it to a boolean or a number using the
//! compile-time constants harvested from the same file.
//! **Position:** third stage of the scrub pipeline; its verdict decides whether an `if` or `while`
//! block is provably dead.
//! **Signals & state:** none. The parser carries only its token run and a borrowed constant table.
//! **Invariants:** a refusal is [`Val::U`] or `None` and is never treated as a value. A token the
//! grammar does not model becomes [`Tok::Other`], which makes the whole expression undecidable
//! rather than silently ignorable.

use super::cfg::cfg_eval;
use super::consts::Consts;
use super::lexer::{balanced, is_ident_char, kw_at};

/// The result of folding an expression: a boolean, a number, or a refusal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Val {
    B(bool),
    N(f64),
    U,
}

/// One token of the miniature expression grammar the constant folder understands.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Tok {
    Ident(String),
    Num(f64),
    Bool(bool),
    Op(&'static str),
    Other,
}

/// Tokenise `expr` into the grammar's tokens. Anything the grammar does not model becomes
/// [`Tok::Other`], which is the evaluator admitting it cannot read that byte.
pub(super) fn lex(expr: &str) -> Vec<Tok> {
    const SUFFIXES: &[&str] = &[
        "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
        "f32", "f64",
    ];
    let c: Vec<char> = expr.chars().collect();
    let mut t = Vec::new();
    let mut i = 0usize;
    while i < c.len() {
        if c[i].is_whitespace() {
            i += 1;
            continue;
        }
        if c[i].is_ascii_digit() {
            let s = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '_' || c[i] == '.') {
                i += 1;
            }
            let ns = i;
            while i < c.len() && is_ident_char(c[i]) {
                i += 1;
            }
            let lit: String = c[s..ns].iter().filter(|x| **x != '_').collect();
            let suffix: String = c[ns..i].iter().collect();
            if !suffix.is_empty() && !SUFFIXES.contains(&suffix.as_str()) {
                t.push(Tok::Other);
                continue;
            }
            t.push(lit.parse::<f64>().map(Tok::Num).unwrap_or(Tok::Other));
            continue;
        }
        // A **leading** `::` is part of the path, not punctuation. `::std::hint::black_box`
        // names the same function as `std::hint::black_box`; lexing the two colons as unknown
        // bytes was enough to make the whole expression undecidable, which used to mean
        // "keep the block". Skipped, not emitted, so the path text still matches a const name.
        let leading_path = c[i] == ':'
            && c.get(i + 1) == Some(&':')
            && c.get(i + 2).is_some_and(|x| is_ident_char(*x));
        if leading_path {
            i += 2;
        }
        if leading_path || is_ident_char(c[i]) {
            let s = i;
            while i < c.len() {
                if is_ident_char(c[i]) {
                    i += 1;
                } else if c[i] == ':' && c.get(i + 1) == Some(&':') {
                    i += 2;
                } else {
                    break;
                }
            }
            let w: String = c[s..i].iter().collect();
            t.push(match w.as_str() {
                "true" => Tok::Bool(true),
                "false" => Tok::Bool(false),
                "as" => Tok::Op("as"),
                _ => Tok::Ident(w),
            });
            continue;
        }
        let two: String = c[i..(i + 2).min(c.len())].iter().collect();
        let two_op = match two.as_str() {
            "&&" => Some("&&"),
            "||" => Some("||"),
            "==" => Some("=="),
            "!=" => Some("!="),
            "<=" => Some("<="),
            ">=" => Some(">="),
            _ => None,
        };
        if let Some(op) = two_op {
            t.push(Tok::Op(op));
            i += 2;
            continue;
        }
        t.push(match c[i] {
            '!' => Tok::Op("!"),
            '<' => Tok::Op("<"),
            '>' => Tok::Op(">"),
            '(' => Tok::Op("("),
            ')' => Tok::Op(")"),
            ',' => Tok::Op(","),
            // `const NEVER: bool = { false };` — a block whose only expression is its tail.
            '{' => Tok::Op("{"),
            '}' => Tok::Op("}"),
            _ => Tok::Other,
        });
        i += 1;
    }
    t
}

/// Recursive-descent reader over a token run, carrying the file's constants for identifier
/// lookup.
struct Parser<'a> {
    t: Vec<Tok>,
    i: usize,
    consts: &'a Consts,
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn eat(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Op(x)) if *x == op) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn or(&mut self) -> Val {
        let mut l = self.and();
        while self.eat("||") {
            let r = self.and();
            l = match (l, r) {
                (Val::B(true), _) | (_, Val::B(true)) => Val::B(true),
                (Val::B(a), Val::B(b)) => Val::B(a || b),
                _ => Val::U,
            };
        }
        l
    }
    fn and(&mut self) -> Val {
        let mut l = self.cmp();
        while self.eat("&&") {
            let r = self.cmp();
            l = match (l, r) {
                (Val::B(false), _) | (_, Val::B(false)) => Val::B(false),
                (Val::B(a), Val::B(b)) => Val::B(a && b),
                _ => Val::U,
            };
        }
        l
    }
    fn cmp(&mut self) -> Val {
        let l = self.unary();
        for op in ["==", "!=", "<=", ">=", "<", ">"] {
            if self.eat(op) {
                let r = self.unary();
                return compare(op, l, r);
            }
        }
        l
    }
    fn unary(&mut self) -> Val {
        if self.eat("!") {
            return match self.unary() {
                Val::B(b) => Val::B(!b),
                _ => Val::U,
            };
        }
        let v = self.primary();
        // `<expr> as bool` / `as u8` — the cast target is an identifier; a non-identifier
        // target is something this pass does not model, so the whole expression is unknown.
        let mut v = v;
        while self.eat("as") {
            match self.peek() {
                Some(Tok::Ident(ty)) => {
                    if ty != "bool" {
                        v = Val::U;
                    }
                    self.i += 1;
                }
                _ => return Val::U,
            }
        }
        v
    }
    fn primary(&mut self) -> Val {
        match self.t.get(self.i).cloned() {
            Some(Tok::Bool(b)) => {
                self.i += 1;
                Val::B(b)
            }
            Some(Tok::Num(n)) => {
                self.i += 1;
                Val::N(n)
            }
            Some(Tok::Op("(")) => {
                self.i += 1;
                let v = self.or();
                if !self.eat(")") {
                    return Val::U;
                }
                v
            }
            // `{ <expr> }` — a block whose value is its tail expression. `const NEVER: bool =
            // { false };` was a survivor purely because `{` lexed as an unknown byte.
            // A block with statements in it stops here and the trailing-token check refuses.
            Some(Tok::Op("{")) => {
                self.i += 1;
                let v = self.or();
                if !self.eat("}") {
                    return Val::U;
                }
                v
            }
            Some(Tok::Ident(name)) => {
                self.i += 1;
                let macro_bang = self.eat("!");
                if self.eat("(") {
                    let mut args = Vec::new();
                    if !self.eat(")") {
                        loop {
                            args.push(self.or());
                            if self.eat(")") {
                                break;
                            }
                            if !self.eat(",") {
                                return Val::U;
                            }
                            if self.eat(")") {
                                break;
                            }
                        }
                    }
                    return if macro_bang {
                        Val::U // `cfg!(…)` is folded before lexing; every other macro is opaque
                    } else {
                        transparent_call(&name, &args)
                    };
                }
                if macro_bang {
                    return Val::U;
                }
                self.consts.known.get(&name).copied().unwrap_or(Val::U)
            }
            _ => {
                self.i = self.t.len();
                Val::U
            }
        }
    }
}

/// Fold a call that is the identity on its argument, so the argument's constness passes through.
///
/// `std::hint::black_box` is the one that matters: it hides a value from the optimiser, not from a
/// reader, and it does not change the value — so folding through it is the correct reading rather
/// than a special case.
pub(super) fn transparent_call(path: &str, args: &[Val]) -> Val {
    let last = path.rsplit("::").next().unwrap_or(path);
    if args.len() == 1 && matches!(last, "black_box" | "identity") {
        return args[0];
    }
    Val::U
}

/// Apply comparison operator `op` to two folded values.
fn compare(op: &str, l: Val, r: Val) -> Val {
    match (l, r) {
        (Val::N(a), Val::N(b)) => Val::B(match op {
            "==" => a == b,
            "!=" => a != b,
            "<=" => a <= b,
            ">=" => a >= b,
            "<" => a < b,
            _ => a > b,
        }),
        (Val::B(a), Val::B(b)) => match op {
            "==" => Val::B(a == b),
            "!=" => Val::B(a != b),
            _ => Val::U,
        },
        _ => Val::U,
    }
}

/// Replace every `cfg!(…)` in `expr` with the literal its predicate evaluates to, so the
/// expression parser never has to model `any` and `all` a second time.
pub(super) fn fold_cfg_macros(expr: &str) -> String {
    let c: Vec<char> = expr.chars().collect();
    let mut out = String::with_capacity(expr.len());
    let mut i = 0usize;
    while i < c.len() {
        if kw_at(&c, i, "cfg") && c.get(i + 3) == Some(&'!') {
            let mut j = i + 4;
            while j < c.len() && c[j].is_whitespace() {
                j += 1;
            }
            if c.get(j) == Some(&'(') {
                if let Some(close) = balanced(&c, j, '(', ')') {
                    let pred: String = c[j + 1..close].iter().collect();
                    match cfg_eval(&pred) {
                        Some(true) => out.push_str("true"),
                        Some(false) => out.push_str("false"),
                        None => out.push_str("__unknown_cfg__"),
                    }
                    i = close + 1;
                    continue;
                }
            }
        }
        out.push(c[i]);
        i += 1;
    }
    out
}

/// Constant-fold `expr` to a boolean **or** a number.
///
/// Numbers are folded too so that `const LIMIT: usize = 5; if LIMIT > 3` resolves instead of being
/// treated as an undecidable constant. `None` is a refusal, never a value.
pub(super) fn eval_value(expr: &str, consts: &Consts) -> Option<Val> {
    let folded = fold_cfg_macros(expr);
    let mut p = Parser {
        t: lex(&folded),
        i: 0,
        consts,
    };
    let v = p.or();
    // Trailing tokens mean the grammar did not describe this expression; refuse rather than
    // act on a partial read — a partial read is exactly the defect this file exists to remove.
    if p.i != p.t.len() {
        return None;
    }
    match v {
        Val::U => None,
        v => Some(v),
    }
}

/// Constant-fold a boolean condition.
///
/// `None` means this evaluator could not read the expression — it does **not** mean the condition
/// is live. Callers must decide the unknown case explicitly.
pub(crate) fn eval_bool(expr: &str, consts: &Consts) -> Option<bool> {
    match eval_value(expr, consts)? {
        Val::B(b) => Some(b),
        _ => None,
    }
}
