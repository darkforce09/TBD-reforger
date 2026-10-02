//! Asset catalog bounded regex behavior.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GlobTok {
    Star,
    AnyOne,
    Ch(char),
}

/// Compiled case-insensitive whole-string glob matcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobPattern {
    toks: Vec<GlobTok>,
}

impl GlobPattern {
    /// Compiles a glob or bounded regular expression pattern.
    pub(super) fn parse(pattern: &str) -> Self {
        let mut toks: Vec<GlobTok> = Vec::new();
        for c in pattern.chars() {
            match c {
                '*' => {
                    if toks.last() != Some(&GlobTok::Star) {
                        toks.push(GlobTok::Star);
                    }
                }
                '?' => toks.push(GlobTok::AnyOne),
                _ => {
                    for lc in c.to_lowercase() {
                        toks.push(GlobTok::Ch(lc));
                    }
                }
            }
        }
        Self { toks }
    }

    /// Matches a complete string against a compiled glob.
    pub(super) fn matches(&self, hay: &str) -> bool {
        let h: Vec<char> = hay.to_lowercase().chars().collect();
        let p = &self.toks;
        let (mut i, mut j) = (0usize, 0usize);
        let mut star: Option<usize> = None;
        let mut mark = 0usize;
        while i < h.len() {
            match p.get(j) {
                Some(GlobTok::Ch(c)) if *c == h[i] => {
                    i += 1;
                    j += 1;
                }
                Some(GlobTok::AnyOne) => {
                    i += 1;
                    j += 1;
                }
                Some(GlobTok::Star) => {
                    star = Some(j);
                    mark = i;
                    j += 1;
                }
                _ => match star {
                    Some(s) => {
                        j = s + 1;
                        mark += 1;
                        i = mark;
                    }
                    None => return false,
                },
            }
        }
        while p.get(j) == Some(&GlobTok::Star) {
            j += 1;
        }
        j == p.len()
    }
}

/// Maximum regular expression matcher steps per query.
pub(super) const RX_BUDGET: u32 = 200_000;

/// Maximum recursive matcher depth per query.
pub(super) const RX_MAX_DEPTH: u32 = 400;

/// Maximum regular expression pattern length.
pub(super) const RX_MAX_PATTERN: usize = 512;

#[derive(Clone, Debug, PartialEq, Eq)]
enum ClassItem {
    Ch(char),
    Range(char, char),
    Digit,
    NotDigit,
    Word,
    NotWord,
    Space,
    NotSpace,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RxNode {
    Ch(char),
    Any,
    Class {
        negated: bool,
        items: Vec<ClassItem>,
    },
    Group(RxAlt),
    Repeat {
        node: Box<RxNode>,
        min: usize,
        max: Option<usize>,
    },
    Start,
    End,
}

/// Alternation branches, each containing a sequence of matcher nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RxAlt(Vec<Vec<RxNode>>);

/// Compiled bounded regular expression matcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rx {
    alt: RxAlt,
}

/// Parser state for the bounded regular expression grammar.
pub(super) struct RxParser<'a> {
    pub(super) src: &'a [char],
    pub(super) pos: usize,
}

impl RxParser<'_> {
    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    /// Parses alternation branches at the current position.
    pub(super) fn alt(&mut self) -> Option<RxAlt> {
        let mut branches = vec![self.seq()?];
        while self.peek() == Some('|') {
            self.pos += 1;
            branches.push(self.seq()?);
        }
        Some(RxAlt(branches))
    }

    fn seq(&mut self) -> Option<Vec<RxNode>> {
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None | Some('|') | Some(')') => break,
                _ => {}
            }
            let atom = self.atom()?;
            out.push(self.quantified(atom));
        }
        Some(out)
    }

    fn quantified(&mut self, node: RxNode) -> RxNode {
        let mut node = node;
        loop {
            let (min, max) = match self.peek() {
                Some('*') => (0, None),
                Some('+') => (1, None),
                Some('?') => (0, Some(1)),
                _ => return node,
            };
            self.pos += 1;
            node = RxNode::Repeat {
                node: Box::new(node),
                min,
                max,
            };
        }
    }

    fn atom(&mut self) -> Option<RxNode> {
        match self.next()? {
            '(' => {
                let alt = self.alt()?;
                (self.next() == Some(')')).then_some(RxNode::Group(alt))
            }
            '[' => self.class(),
            '.' => Some(RxNode::Any),
            '^' => Some(RxNode::Start),
            '$' => Some(RxNode::End),
            '\\' => self.escape(),
            '*' | '+' | '?' => None,
            c => Some(RxNode::Ch(c.to_ascii_lowercase())),
        }
    }

    fn escape(&mut self) -> Option<RxNode> {
        let c = self.next()?;
        Some(match class_shorthand(c) {
            Some(item) => RxNode::Class {
                negated: false,
                items: vec![item],
            },
            None => RxNode::Ch(c.to_ascii_lowercase()),
        })
    }

    fn class(&mut self) -> Option<RxNode> {
        let negated = self.peek() == Some('^');
        if negated {
            self.pos += 1;
        }
        let mut items = Vec::new();
        loop {
            let c = self.next()?; // unterminated class ⇒ None ⇒ Invalid
            if c == ']' && !items.is_empty() {
                return Some(RxNode::Class { negated, items });
            }
            let lo = if c == '\\' {
                let e = self.next()?;
                if let Some(item) = class_shorthand(e) {
                    items.push(item);
                    continue;
                }
                e
            } else {
                c
            };
            if self.peek() == Some('-') && self.src.get(self.pos + 1).is_some_and(|n| *n != ']') {
                self.pos += 1;
                let hi = self.next()?;
                items.push(ClassItem::Range(
                    lo.to_ascii_lowercase(),
                    hi.to_ascii_lowercase(),
                ));
            } else {
                items.push(ClassItem::Ch(lo));
            }
        }
    }
}

fn class_shorthand(c: char) -> Option<ClassItem> {
    Some(match c {
        'd' => ClassItem::Digit,
        'D' => ClassItem::NotDigit,
        'w' => ClassItem::Word,
        'W' => ClassItem::NotWord,
        's' => ClassItem::Space,
        'S' => ClassItem::NotSpace,
        _ => return None,
    })
}

fn eq_ci(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

fn class_hit(items: &[ClassItem], h: char) -> bool {
    items.iter().any(|it| match it {
        ClassItem::Ch(c) => eq_ci(*c, h),
        ClassItem::Range(a, b) => {
            (*a..=*b).contains(&h) || (*a..=*b).contains(&h.to_ascii_uppercase())
        }
        ClassItem::Digit => h.is_ascii_digit(),
        ClassItem::NotDigit => !h.is_ascii_digit(),
        ClassItem::Word => h.is_alphanumeric() || h == '_',
        ClassItem::NotWord => !(h.is_alphanumeric() || h == '_'),
        ClassItem::Space => h.is_whitespace(),
        ClassItem::NotSpace => !h.is_whitespace(),
    })
}

type RxCont<'a> = &'a dyn Fn(usize) -> bool;

struct RxCtx<'h> {
    hay: &'h [char],
    budget: std::cell::Cell<u32>,
    depth: std::cell::Cell<u32>,
    depth_capped: std::cell::Cell<bool>,
}

impl RxCtx<'_> {
    fn step(&self) -> bool {
        let b = self.budget.get();
        if b == 0 {
            return false;
        }
        self.budget.set(b - 1);
        true
    }

    fn alt(&self, a: &RxAlt, pos: usize, k: RxCont) -> bool {
        self.step() && a.0.iter().any(|s| self.seq(s, pos, k))
    }

    fn seq(&self, s: &[RxNode], pos: usize, k: RxCont) -> bool {
        match s.split_first() {
            None => k(pos),
            Some((n, rest)) => self.node(n, pos, &|p| self.seq(rest, p, k)),
        }
    }

    fn node(&self, n: &RxNode, pos: usize, k: RxCont) -> bool {
        if !self.step() {
            return false;
        }
        let d = self.depth.get();
        if d >= RX_MAX_DEPTH {
            self.depth_capped.set(true);
            return false;
        }
        self.depth.set(d + 1);
        let hit = self.node_inner(n, pos, k);
        self.depth.set(d);
        hit
    }

    fn node_inner(&self, n: &RxNode, pos: usize, k: RxCont) -> bool {
        match n {
            RxNode::Start => pos == 0 && k(pos),
            RxNode::End => pos == self.hay.len() && k(pos),
            RxNode::Ch(c) => self.hay.get(pos).is_some_and(|h| eq_ci(*h, *c)) && k(pos + 1),
            RxNode::Any => pos < self.hay.len() && k(pos + 1),
            RxNode::Class { negated, items } => {
                self.hay
                    .get(pos)
                    .is_some_and(|h| class_hit(items, *h) != *negated)
                    && k(pos + 1)
            }
            RxNode::Group(a) => self.alt(a, pos, k),
            RxNode::Repeat { node, min, max } => self.repeat(node, *min, *max, pos, 0, k),
        }
    }

    fn repeat(
        &self,
        node: &RxNode,
        min: usize,
        max: Option<usize>,
        pos: usize,
        count: usize,
        k: RxCont,
    ) -> bool {
        if !self.step() {
            return false;
        }
        if max.is_none_or(|m| count < m)
            && self.node(node, pos, &|p| {
                if p == pos {
                    count + 1 >= min && k(p)
                } else {
                    self.repeat(node, min, max, p, count + 1, k)
                }
            })
        {
            return true;
        }
        count >= min && k(pos)
    }
}

impl Rx {
    /// Compiles a glob or bounded regular expression pattern.
    pub(super) fn parse(pattern: &str) -> Option<Self> {
        let src: Vec<char> = pattern.chars().collect();
        if src.len() > RX_MAX_PATTERN {
            return None;
        }
        let mut p = RxParser { src: &src, pos: 0 };
        let alt = p.alt()?;
        (p.pos == src.len()).then_some(Self { alt })
    }

    /// Reports whether the expression matches anywhere in a string.
    pub(super) fn is_match(&self, hay: &str) -> bool {
        self.search(hay).0
    }

    /// Returns the match result and whether the depth cap applied.
    pub(super) fn search(&self, hay: &str) -> (bool, bool) {
        let h: Vec<char> = hay.to_lowercase().chars().collect();
        let ctx = RxCtx {
            hay: &h,
            budget: std::cell::Cell::new(RX_BUDGET),
            depth: std::cell::Cell::new(0),
            depth_capped: std::cell::Cell::new(false),
        };
        let hit = (0..=h.len()).any(|start| ctx.alt(&self.alt, start, &|_| true));
        (hit, ctx.depth_capped.get())
    }
}
