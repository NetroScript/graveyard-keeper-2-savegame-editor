use crate::service::view;
use crate::wire::{base_tag, Document, Error, Payload, TypeInfo};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

const MAX_QUERY_BYTES: usize = 8 * 1024;
const MAX_TERMS: usize = 512;
const MAX_GROUP_DEPTH: usize = 32;
const MAX_CONTEXTS: usize = 2_000_000;
const MAX_ROUTE_DEPTH: usize = 256;
const STEP_CONTEXTS: usize = 2_048;
const MAX_RELATION_CACHE_ENTRIES: usize = 262_144;

#[derive(Clone, Copy, Debug)]
enum Field {
    Any,
    Name,
    Path,
    Type,
    Class,
    Owner,
    Ancestor,
    Value,
}

#[derive(Clone, Debug)]
enum Expr {
    Text(Field, Pattern),
    Exact(Field, Pattern),
    Number(Cmp, Decimal),
    Relation(Relation, u16, Box<Expr>),
    Not(Box<Expr>),
    And(Vec<Expr>),
    Or(Vec<Expr>),
}

#[derive(Clone, Copy, Debug)]
enum Relation {
    Child,
    Descendant,
    Parent,
}

impl Relation {
    fn label(self) -> &'static str {
        match self {
            Self::Child => "child",
            Self::Descendant => "descendant",
            Self::Parent => "parent",
        }
    }
}

#[derive(Clone, Debug)]
struct Pattern {
    text: String,
    glob: bool,
}

#[derive(Clone, Copy, Debug)]
enum Cmp {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Decimal {
    negative: bool,
    digits: String,
    scale: i64,
}

impl Decimal {
    fn parse(input: &str) -> Option<Self> {
        let mut s = input;
        let negative = s
            .strip_prefix('-')
            .map(|v| {
                s = v;
                true
            })
            .unwrap_or_else(|| {
                if let Some(v) = s.strip_prefix('+') {
                    s = v;
                }
                false
            });
        let (mantissa, exponent) = match s.find(['e', 'E']) {
            Some(at) => (&s[..at], s[at + 1..].parse::<i64>().ok()?),
            None => (s, 0),
        };
        let mut digits = String::new();
        let mut fractional = 0i64;
        let mut dot = false;
        for ch in mantissa.chars() {
            if ch == '.' && !dot {
                dot = true;
                continue;
            }
            if !ch.is_ascii_digit() {
                return None;
            }
            digits.push(ch);
            if dot {
                fractional += 1;
            }
        }
        if digits.is_empty() {
            return None;
        }
        let trimmed = digits.trim_start_matches('0');
        digits = if trimmed.is_empty() {
            "0".into()
        } else {
            trimmed.into()
        };
        let mut value = Self {
            negative: negative && digits != "0",
            digits,
            scale: fractional - exponent,
        };
        while value.scale > 0 && value.digits.ends_with('0') {
            value.digits.pop();
            value.scale -= 1;
        }
        Some(value)
    }
    fn cmp_abs(&self, other: &Self) -> Ordering {
        let left_int = self.digits.len() as i64 - self.scale;
        let right_int = other.digits.len() as i64 - other.scale;
        match left_int.cmp(&right_int) {
            Ordering::Equal => {
                let scale = self.scale.max(other.scale).max(0);
                let mut left = self.digits.clone();
                let mut right = other.digits.clone();
                left.extend(std::iter::repeat('0').take((scale - self.scale).max(0) as usize));
                right.extend(std::iter::repeat('0').take((scale - other.scale).max(0) as usize));
                left.cmp(&right)
            }
            order => order,
        }
    }
    fn compare(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (true, true) => self.cmp_abs(other).reverse(),
            _ => self.cmp_abs(other),
        }
    }
}

#[derive(Clone)]
struct Route {
    id: usize,
    path: String,
    ancestors: Vec<String>,
    objects: Vec<usize>,
    depth: usize,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResult {
    pub node: usize,
    pub name: String,
    pub kind: String,
    pub type_name: Option<String>,
    pub value: Option<String>,
    pub path: String,
    pub match_field: String,
    #[serde(skip)]
    score: u16,
}

pub(crate) struct SearchState {
    id: u32,
    revision: u32,
    expression: Expr,
    case_sensitive: bool,
    stack: Vec<Route>,
    results: Vec<SearchResult>,
    matched: HashSet<usize>,
    relation_cache: HashMap<(u16, usize), bool>,
    visited: usize,
    complete: bool,
    incomplete: bool,
    reason: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchStatus {
    search_id: u32,
    revision: u32,
    status: &'static str,
    discovered: usize,
    visited: usize,
    completion_reason: Option<String>,
}

impl SearchState {
    pub fn new(
        doc: &Document,
        revision: u32,
        id: u32,
        query: &str,
        case_sensitive: bool,
    ) -> Result<Self, Error> {
        let expression = Parser::new(query)?.parse()?;
        let stack = doc
            .roots
            .iter()
            .enumerate()
            .rev()
            .map(|(index, node)| Route {
                id: *node,
                path: segment(&doc.records[*node], index),
                ancestors: vec![],
                objects: vec![],
                depth: 0,
            })
            .collect();
        Ok(Self {
            id,
            revision,
            expression,
            case_sensitive,
            stack,
            results: vec![],
            matched: HashSet::new(),
            relation_cache: HashMap::new(),
            visited: 0,
            complete: false,
            incomplete: false,
            reason: None,
        })
    }
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn revision(&self) -> u32 {
        self.revision
    }
    pub fn status(&self) -> SearchStatus {
        SearchStatus {
            search_id: self.id,
            revision: self.revision,
            status: if !self.complete {
                "searching"
            } else if self.incomplete {
                "incomplete"
            } else {
                "complete"
            },
            discovered: self.results.len(),
            visited: self.visited,
            completion_reason: self.reason.clone(),
        }
    }
    pub fn page(&self, offset: usize, limit: usize) -> &[SearchResult] {
        &self.results
            [offset.min(self.results.len())..offset.saturating_add(limit).min(self.results.len())]
    }
    pub fn step(&mut self, doc: &Document) {
        if self.complete {
            return;
        }
        let mut processed = 0;
        while processed < STEP_CONTEXTS {
            if self.visited >= MAX_CONTEXTS {
                self.incomplete = true;
                self.reason = Some(
                    "Search stopped after two million record contexts. Narrow the query.".into(),
                );
                self.complete = true;
                self.sort_results();
                return;
            }
            let Some(route) = self.stack.pop() else {
                self.complete = true;
                self.sort_results();
                return;
            };
            self.visited += 1;
            processed += 1;
            let record = &doc.records[route.id];
            let own_class = type_name(doc, route.id);
            let candidate = Candidate {
                doc,
                route: &route,
                own_class: own_class.as_deref(),
            };
            if evaluate(
                &self.expression,
                &candidate,
                self.case_sensitive,
                &mut self.relation_cache,
            ) && self.matched.insert(route.id)
            {
                let relevance = relevance(
                    &self.expression,
                    &candidate,
                    self.case_sensitive,
                    &mut self.relation_cache,
                )
                .unwrap_or(Relevance {
                    score: 0,
                    field: "query",
                });
                let node = view(doc, route.id);
                self.results.push(SearchResult {
                    node: route.id,
                    name: node.name.unwrap_or_else(|| node.kind.clone()),
                    kind: node.kind,
                    type_name: node.type_name,
                    value: node.value.map(|v| truncate(&v, 160)),
                    path: route.path.clone(),
                    match_field: relevance.field.into(),
                    score: relevance.score,
                });
            }
            let mut ancestors = route.ancestors.clone();
            if let Some(class) = own_class {
                ancestors.push(class);
            }
            if route.depth >= MAX_ROUTE_DEPTH {
                if !record.children.is_empty() || base_tag(record.tag) == 10 {
                    self.incomplete = true;
                    self.reason.get_or_insert_with(|| {
                        "Some routes exceeded the search depth limit of 256.".into()
                    });
                }
                continue;
            }
            for (index, child) in record.children.iter().enumerate().rev() {
                self.stack.push(Route {
                    id: *child,
                    path: join_path(&route.path, &segment(&doc.records[*child], index)),
                    ancestors: ancestors.clone(),
                    objects: route.objects.clone(),
                    depth: route.depth + 1,
                });
            }
            if base_tag(record.tag) == 10 {
                if let Payload::Fixed(bytes) = &record.payload {
                    if bytes.len() == 4 {
                        let object = i32::from_le_bytes(bytes.as_slice().try_into().unwrap());
                        if let Some(target) = doc.objects.get(&object).copied() {
                            if !route.objects.contains(&target) {
                                let mut objects = route.objects.clone();
                                objects.push(target);
                                let target_name = doc.records[target]
                                    .name
                                    .as_ref()
                                    .map(|v| v.display())
                                    .unwrap_or_else(|| format!("object#{object}"));
                                self.stack.push(Route {
                                    id: target,
                                    path: format!("{} → {}", route.path, escape_path(&target_name)),
                                    ancestors,
                                    objects,
                                    depth: route.depth + 1,
                                });
                            }
                        }
                    }
                }
            }
        }
        if self.stack.is_empty() {
            self.complete = true;
            self.sort_results();
        }
    }
    fn sort_results(&mut self) {
        self.results.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.node.cmp(&right.node))
        });
    }
}

fn truncate(value: &str, max: usize) -> String {
    let mut chars = value.chars();
    let text: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        format!("{text}…")
    } else {
        text
    }
}
fn escape_path(value: &str) -> String {
    value.replace('\\', "\\\\").replace('/', "\\/")
}
fn join_path(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.into()
    } else {
        format!("{parent}/{child}")
    }
}
fn segment(record: &crate::wire::Record, index: usize) -> String {
    record
        .name
        .as_ref()
        .map(|v| escape_path(&v.display()))
        .unwrap_or_else(|| format!("[{index}]"))
}
fn type_name(doc: &Document, id: usize) -> Option<String> {
    let Payload::Node { ty, .. } = &doc.records[id].payload else {
        return None;
    };
    let id = match ty {
        TypeInfo::Definition(id, _) | TypeInfo::Reference(id) => id,
        TypeInfo::None => return None,
    };
    doc.types.get(id).cloned()
}
fn short_class(value: &str) -> &str {
    value
        .split(',')
        .next()
        .unwrap_or(value)
        .trim()
        .rsplit('.')
        .next()
        .unwrap_or(value)
}
fn class_name(value: &str) -> &str {
    value.split(',').next().unwrap_or(value).trim()
}

struct Candidate<'a> {
    doc: &'a Document,
    route: &'a Route,
    own_class: Option<&'a str>,
}

fn evaluate(
    expr: &Expr,
    c: &Candidate<'_>,
    case_sensitive: bool,
    relation_cache: &mut HashMap<(u16, usize), bool>,
) -> bool {
    match expr {
        Expr::Text(field, pattern) => evaluate_text(*field, pattern, c, case_sensitive, false),
        Expr::Exact(field, pattern) => evaluate_text(*field, pattern, c, case_sensitive, true),
        Expr::Number(op, expected) => {
            numeric_value(c.doc, c.route.id).is_some_and(|actual| match op {
                Cmp::Eq => actual.compare(expected) == Ordering::Equal,
                Cmp::Lt => actual.compare(expected) == Ordering::Less,
                Cmp::Le => actual.compare(expected) != Ordering::Greater,
                Cmp::Gt => actual.compare(expected) == Ordering::Greater,
                Cmp::Ge => actual.compare(expected) != Ordering::Less,
            })
        }
        Expr::Relation(relation, id, inner) => relation_matches(
            *relation,
            *id,
            inner,
            c.doc,
            c.route.id,
            case_sensitive,
            relation_cache,
        ),
        Expr::Not(value) => !evaluate(value, c, case_sensitive, relation_cache),
        Expr::And(values) => values
            .iter()
            .all(|v| evaluate(v, c, case_sensitive, relation_cache)),
        Expr::Or(values) => values
            .iter()
            .any(|v| evaluate(v, c, case_sensitive, relation_cache)),
    }
}

fn relation_matches(
    relation: Relation,
    relation_id: u16,
    inner: &Expr,
    doc: &Document,
    id: usize,
    case_sensitive: bool,
    relation_cache: &mut HashMap<(u16, usize), bool>,
) -> bool {
    match relation {
        Relation::Child => doc.records[id]
            .children
            .iter()
            .copied()
            .any(|child| evaluate_local(inner, doc, child, case_sensitive, relation_cache)),
        Relation::Parent => doc.records[id].parent.is_some_and(|parent| {
            evaluate_local(inner, doc, parent, case_sensitive, relation_cache)
        }),
        Relation::Descendant => {
            if let Some(result) = relation_cache.get(&(relation_id, id)) {
                return *result;
            }
            let result = doc.records[id].children.iter().copied().any(|child| {
                evaluate_local(inner, doc, child, case_sensitive, relation_cache)
                    || relation_matches(
                        relation,
                        relation_id,
                        inner,
                        doc,
                        child,
                        case_sensitive,
                        relation_cache,
                    )
            });
            if relation_cache.len() < MAX_RELATION_CACHE_ENTRIES {
                relation_cache.insert((relation_id, id), result);
            }
            result
        }
    }
}

fn evaluate_local(
    expr: &Expr,
    doc: &Document,
    id: usize,
    case_sensitive: bool,
    relation_cache: &mut HashMap<(u16, usize), bool>,
) -> bool {
    let route = Route {
        id,
        path: String::new(),
        ancestors: vec![],
        objects: vec![],
        depth: 0,
    };
    let own_class = type_name(doc, id);
    let candidate = Candidate {
        doc,
        route: &route,
        own_class: own_class.as_deref(),
    };
    evaluate(expr, &candidate, case_sensitive, relation_cache)
}

fn evaluate_text(
    field: Field,
    pattern: &Pattern,
    c: &Candidate<'_>,
    case_sensitive: bool,
    exact: bool,
) -> bool {
    match field {
        Field::Name => matches_text(
            c.doc.records[c.route.id]
                .name
                .as_ref()
                .map(|v| v.display())
                .as_deref(),
            pattern,
            case_sensitive,
            exact,
        ),
        Field::Path => matches_text(Some(&c.route.path), pattern, case_sensitive, exact),
        Field::Type => matches_type(
            base_tag(c.doc.records[c.route.id].tag),
            pattern,
            case_sensitive,
        ),
        Field::Class => matches_class(c.own_class.into_iter(), pattern, case_sensitive),
        Field::Owner => matches_class(
            c.route.ancestors.last().map(String::as_str).into_iter(),
            pattern,
            case_sensitive,
        ),
        Field::Ancestor => matches_class(
            c.route.ancestors.iter().map(String::as_str),
            pattern,
            case_sensitive,
        ),
        Field::Value => matches_text(
            view(c.doc, c.route.id).value.as_deref(),
            pattern,
            case_sensitive,
            exact,
        ),
        Field::Any => {
            let node = view(c.doc, c.route.id);
            matches_text(node.name.as_deref(), pattern, case_sensitive, exact)
                || matches_text(node.value.as_deref(), pattern, case_sensitive, exact)
                || matches_text(Some(&node.kind), pattern, case_sensitive, exact)
                || matches_text(Some(&c.route.path), pattern, case_sensitive, exact)
                || matches_class(c.own_class.into_iter(), pattern, case_sensitive)
        }
    }
}

#[derive(Clone, Copy)]
struct Relevance {
    score: u16,
    field: &'static str,
}

fn relevance(
    expr: &Expr,
    c: &Candidate<'_>,
    case_sensitive: bool,
    relation_cache: &mut HashMap<(u16, usize), bool>,
) -> Option<Relevance> {
    if !evaluate(expr, c, case_sensitive, relation_cache) {
        return None;
    }
    match expr {
        Expr::Text(field, pattern) | Expr::Exact(field, pattern) => {
            relevance_text(*field, pattern, c, case_sensitive)
        }
        Expr::Number(_, _) => Some(Relevance {
            score: 650,
            field: "value",
        }),
        Expr::Relation(relation, _, _) => Some(Relevance {
            score: 450,
            field: relation.label(),
        }),
        Expr::Not(_) => None,
        Expr::And(values) | Expr::Or(values) => values
            .iter()
            .filter_map(|value| relevance(value, c, case_sensitive, relation_cache))
            .max_by_key(|value| value.score),
    }
}

fn relevance_text(
    field: Field,
    pattern: &Pattern,
    c: &Candidate<'_>,
    case_sensitive: bool,
) -> Option<Relevance> {
    let text = |value: Option<&str>, base, field| {
        text_quality(value, pattern, case_sensitive).map(|quality| Relevance {
            score: base + quality,
            field,
        })
    };
    match field {
        Field::Name => text(
            c.doc.records[c.route.id]
                .name
                .as_ref()
                .map(|value| value.display())
                .as_deref(),
            600,
            "name",
        ),
        Field::Path => text(Some(&c.route.path), 300, "path"),
        Field::Type => text(
            Some(view_kind(base_tag(c.doc.records[c.route.id].tag))),
            500,
            "type",
        ),
        Field::Class => class_relevance(c.own_class.into_iter(), pattern, case_sensitive, 550),
        Field::Owner => class_relevance(
            c.route.ancestors.last().map(String::as_str).into_iter(),
            pattern,
            case_sensitive,
            550,
        ),
        Field::Ancestor => class_relevance(
            c.route.ancestors.iter().map(String::as_str),
            pattern,
            case_sensitive,
            500,
        ),
        Field::Value => text(view(c.doc, c.route.id).value.as_deref(), 700, "value"),
        Field::Any => {
            let node = view(c.doc, c.route.id);
            [
                text(node.value.as_deref(), 700, "value"),
                text(node.name.as_deref(), 600, "name"),
                class_relevance(c.own_class.into_iter(), pattern, case_sensitive, 550),
                text(Some(&node.kind), 500, "type"),
                text(Some(&c.route.path), 100, "path"),
            ]
            .into_iter()
            .flatten()
            .max_by_key(|value| value.score)
        }
    }
}

fn class_relevance<'a>(
    values: impl Iterator<Item = &'a str>,
    pattern: &Pattern,
    case_sensitive: bool,
    base: u16,
) -> Option<Relevance> {
    values
        .flat_map(|value| [value, class_name(value), short_class(value)])
        .filter_map(|value| text_quality(Some(value), pattern, case_sensitive))
        .max()
        .map(|quality| Relevance {
            score: base + quality,
            field: "class",
        })
}

fn text_quality(value: Option<&str>, pattern: &Pattern, case_sensitive: bool) -> Option<u16> {
    let value = value?;
    let (value, needle) = if case_sensitive {
        (value.to_owned(), pattern.text.clone())
    } else {
        (value.to_lowercase(), pattern.text.to_lowercase())
    };
    if pattern.glob {
        return glob(&value, &needle).then_some(30);
    }
    if value == needle {
        Some(100)
    } else if value.starts_with(&needle) {
        Some(70)
    } else if value.contains(&needle) {
        Some(20)
    } else {
        None
    }
}

fn matches_type(tag: u8, pattern: &Pattern, case_sensitive: bool) -> bool {
    let kind = view_kind(tag);
    let alias = match pattern.text.to_ascii_lowercase().as_str() {
        "int" => matches!(tag, 16 | 18 | 20 | 22 | 24 | 26 | 28 | 30),
        "float" => matches!(tag, 32 | 34),
        "number" => matches!(tag, 16 | 18 | 20 | 22 | 24 | 26 | 28 | 30 | 32 | 34 | 36),
        _ => false,
    };
    alias || matches_text(Some(kind), pattern, case_sensitive, true)
}
fn view_kind(tag: u8) -> &'static str {
    match base_tag(tag) {
        1 | 2 => "reference_node",
        3 | 4 => "struct",
        6 => "array",
        8 => "primitive_array",
        10 => "internal_reference",
        12 | 14 | 50 | 51 => "external_reference",
        16 => "i8",
        18 => "u8",
        20 => "i16",
        22 => "u16",
        24 => "i32",
        26 => "u32",
        28 => "i64",
        30 => "u64",
        32 => "f32",
        34 => "f64",
        36 => "decimal",
        38 => "char",
        40 => "string",
        42 => "guid",
        44 => "bool",
        46 => "null",
        _ => "end",
    }
}
fn matches_class<'a>(
    values: impl Iterator<Item = &'a str>,
    pattern: &Pattern,
    case_sensitive: bool,
) -> bool {
    values.into_iter().any(|value| {
        let exact = !pattern.glob;
        matches_text(Some(value), pattern, case_sensitive, exact)
            || matches_text(Some(class_name(value)), pattern, case_sensitive, exact)
            || matches_text(Some(short_class(value)), pattern, case_sensitive, exact)
    })
}
fn matches_text(value: Option<&str>, pattern: &Pattern, case_sensitive: bool, exact: bool) -> bool {
    let Some(value) = value else {
        return false;
    };
    let (value, needle) = if case_sensitive {
        (value.to_owned(), pattern.text.clone())
    } else {
        (value.to_lowercase(), pattern.text.to_lowercase())
    };
    if pattern.glob {
        glob(&value, &needle)
    } else if exact {
        value == needle
    } else {
        value.contains(&needle)
    }
}
fn glob(value: &str, pattern: &str) -> bool {
    let value: Vec<char> = value.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    let (mut vi, mut pi, mut star, mut retry) = (0, 0, None, 0);
    while vi < value.len() {
        if pi < pattern.len() && (pattern[pi] == '?' || pattern[pi] == value[vi]) {
            vi += 1;
            pi += 1;
        } else if pi < pattern.len() && pattern[pi] == '*' {
            star = Some(pi);
            retry = vi;
            pi += 1;
        } else if let Some(at) = star {
            retry += 1;
            vi = retry;
            pi = at + 1;
        } else {
            return false;
        }
    }
    while pi < pattern.len() && pattern[pi] == '*' {
        pi += 1;
    }
    pi == pattern.len()
}

fn numeric_value(doc: &Document, id: usize) -> Option<Decimal> {
    let record = &doc.records[id];
    let Payload::Fixed(bytes) = &record.payload else {
        return None;
    };
    macro_rules! number {
        ($ty:ty) => {
            Decimal::parse(&<$ty>::from_le_bytes(bytes.as_slice().try_into().ok()?).to_string())
        };
    }
    match base_tag(record.tag) {
        16 => number!(i8),
        18 => number!(u8),
        20 => number!(i16),
        22 => number!(u16),
        24 => number!(i32),
        26 => number!(u32),
        28 => number!(i64),
        30 => number!(u64),
        32 => {
            let n = f32::from_le_bytes(bytes.as_slice().try_into().ok()?);
            n.is_finite()
                .then(|| Decimal::parse(&n.to_string()))
                .flatten()
        }
        34 => {
            let n = f64::from_le_bytes(bytes.as_slice().try_into().ok()?);
            n.is_finite()
                .then(|| Decimal::parse(&n.to_string()))
                .flatten()
        }
        36 if bytes.len() == 16 => {
            let lo = u32::from_le_bytes(bytes[0..4].try_into().ok()?) as u128;
            let mid = u32::from_le_bytes(bytes[4..8].try_into().ok()?) as u128;
            let hi = u32::from_le_bytes(bytes[8..12].try_into().ok()?) as u128;
            let flags = u32::from_le_bytes(bytes[12..16].try_into().ok()?);
            let scale = ((flags >> 16) & 0xff) as i64;
            let digits = (lo | mid << 32 | hi << 64).to_string();
            Some(Decimal {
                negative: flags >> 31 == 1 && digits != "0",
                digits,
                scale,
            })
        }
        _ => None,
    }
}

#[derive(Clone, Debug)]
enum TokenKind {
    Word(String, bool),
    LParen,
    RParen,
    Or,
    Not,
    Colon,
    Exact,
    Compare(Cmp),
    End,
}
#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    pos: usize,
}

struct Lexer<'a> {
    input: &'a str,
    at: usize,
}
impl<'a> Lexer<'a> {
    fn next(&mut self) -> Result<Token, Error> {
        while self.input[self.at..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            self.at += self.input[self.at..].chars().next().unwrap().len_utf8();
        }
        let pos = self.at;
        let Some(ch) = self.input[self.at..].chars().next() else {
            return Ok(Token {
                kind: TokenKind::End,
                pos,
            });
        };
        self.at += ch.len_utf8();
        let single = match ch {
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            '|' => Some(TokenKind::Or),
            '!' => Some(TokenKind::Not),
            ':' => Some(TokenKind::Colon),
            _ => None,
        };
        if let Some(kind) = single {
            return Ok(Token { kind, pos });
        }
        if ch == '=' && self.input[self.at..].starts_with('=') {
            self.at += 1;
            return Ok(Token {
                kind: TokenKind::Exact,
                pos,
            });
        }
        if matches!(ch, '=' | '<' | '>') {
            let equal = self.input[self.at..].starts_with('=');
            if equal {
                self.at += 1;
            }
            let cmp = match (ch, equal) {
                ('=', _) => Cmp::Eq,
                ('<', false) => Cmp::Lt,
                ('<', true) => Cmp::Le,
                ('>', false) => Cmp::Gt,
                ('>', true) => Cmp::Ge,
                _ => unreachable!(),
            };
            return Ok(Token {
                kind: TokenKind::Compare(cmp),
                pos,
            });
        }
        if ch == '"' {
            let mut value = String::new();
            loop {
                let Some(next) = self.input[self.at..].chars().next() else {
                    return Err(parse_error(pos, "Unterminated quoted text"));
                };
                self.at += next.len_utf8();
                if next == '"' {
                    break;
                }
                if next == '\\' {
                    let Some(escaped) = self.input[self.at..].chars().next() else {
                        return Err(parse_error(self.at, "Incomplete escape"));
                    };
                    if !matches!(escaped, '"' | '\\') {
                        return Err(parse_error(
                            self.at,
                            "Only quote and backslash can be escaped",
                        ));
                    }
                    self.at += escaped.len_utf8();
                    value.push(escaped);
                } else {
                    value.push(next);
                }
            }
            return Ok(Token {
                kind: TokenKind::Word(value, false),
                pos,
            });
        }
        let mut value = String::from(ch);
        let mut glob = matches!(ch, '*' | '?');
        while let Some(next) = self.input[self.at..].chars().next() {
            if next.is_whitespace() || matches!(next, '(' | ')' | '|' | '!' | ':' | '=' | '<' | '>')
            {
                break;
            }
            self.at += next.len_utf8();
            value.push(next);
            glob |= matches!(next, '*' | '?');
        }
        Ok(Token {
            kind: TokenKind::Word(value, glob),
            pos,
        })
    }
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    terms: usize,
    relations: u16,
}
impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Result<Self, Error> {
        if input.len() > MAX_QUERY_BYTES {
            return Err(Error("Search query exceeds 8 KiB".into()));
        }
        let mut lexer = Lexer { input, at: 0 };
        let current = lexer.next()?;
        Ok(Self {
            lexer,
            current,
            terms: 0,
            relations: 0,
        })
    }
    fn bump(&mut self) -> Result<Token, Error> {
        let old = self.current.clone();
        self.current = self.lexer.next()?;
        Ok(old)
    }
    fn parse(mut self) -> Result<Expr, Error> {
        if matches!(self.current.kind, TokenKind::End) {
            return Err(parse_error(0, "Enter a search query"));
        }
        let value = self.or(0, None)?;
        if !matches!(self.current.kind, TokenKind::End) {
            return Err(parse_error(self.current.pos, "Unexpected token"));
        }
        Ok(value)
    }
    fn or(&mut self, depth: usize, field: Option<(Field, bool)>) -> Result<Expr, Error> {
        let mut values = vec![self.and(depth, field)?];
        while matches!(self.current.kind, TokenKind::Or) {
            self.bump()?;
            values.push(self.and(depth, field)?);
        }
        Ok(if values.len() == 1 {
            values.pop().unwrap()
        } else {
            Expr::Or(values)
        })
    }
    fn and(&mut self, depth: usize, field: Option<(Field, bool)>) -> Result<Expr, Error> {
        let mut values = vec![];
        while !matches!(
            self.current.kind,
            TokenKind::End | TokenKind::RParen | TokenKind::Or
        ) {
            values.push(self.unary(depth, field)?);
        }
        if values.is_empty() {
            return Err(parse_error(self.current.pos, "Expected a search term"));
        }
        Ok(if values.len() == 1 {
            values.pop().unwrap()
        } else {
            Expr::And(values)
        })
    }
    fn unary(&mut self, depth: usize, field: Option<(Field, bool)>) -> Result<Expr, Error> {
        if matches!(self.current.kind, TokenKind::Not) {
            self.bump()?;
            return Ok(Expr::Not(Box::new(self.unary(depth, field)?)));
        }
        self.primary(depth, field)
    }
    fn primary(&mut self, depth: usize, inherited: Option<(Field, bool)>) -> Result<Expr, Error> {
        if matches!(self.current.kind, TokenKind::LParen) {
            if depth >= MAX_GROUP_DEPTH {
                return Err(parse_error(
                    self.current.pos,
                    "Search grouping exceeds 32 levels",
                ));
            }
            self.bump()?;
            let value = self.or(depth + 1, inherited)?;
            if !matches!(self.current.kind, TokenKind::RParen) {
                return Err(parse_error(self.current.pos, "Expected )"));
            }
            self.bump()?;
            return Ok(value);
        }
        let token = self.bump()?;
        let TokenKind::Word(text, glob) = token.kind else {
            return Err(parse_error(token.pos, "Expected a search term"));
        };
        if let TokenKind::Compare(cmp) = self.current.kind {
            if inherited.is_some() || text.to_ascii_lowercase() != "value" {
                return Err(parse_error(
                    token.pos,
                    "Numeric comparisons must begin with value",
                ));
            }
            self.bump()?;
            let number = self.bump()?;
            let TokenKind::Word(raw, false) = number.kind else {
                return Err(parse_error(number.pos, "Expected a numeric literal"));
            };
            let value = Decimal::parse(&raw)
                .ok_or_else(|| parse_error(number.pos, "Invalid numeric literal"))?;
            self.term()?;
            return Ok(Expr::Number(cmp, value));
        }
        if matches!(self.current.kind, TokenKind::Colon) && inherited.is_none() {
            self.bump()?;
            if let Some(relation) = parse_relation(&text) {
                if depth >= MAX_GROUP_DEPTH {
                    return Err(parse_error(token.pos, "Search grouping exceeds 32 levels"));
                }
                if !matches!(self.current.kind, TokenKind::LParen) {
                    return Err(parse_error(
                        self.current.pos,
                        "Structural predicates require a grouped expression",
                    ));
                }
                self.bump()?;
                let value = self.or(depth + 1, None)?;
                if !matches!(self.current.kind, TokenKind::RParen) {
                    return Err(parse_error(self.current.pos, "Expected )"));
                }
                self.bump()?;
                if !is_local_expression(&value) {
                    return Err(parse_error(
                        token.pos,
                        "Structural predicates only support name, type, class, value, numeric comparisons and nested structural predicates",
                    ));
                }
                self.term()?;
                let id = self.relations;
                self.relations += 1;
                return Ok(Expr::Relation(relation, id, Box::new(value)));
            }
            let field = parse_field(&text)
                .ok_or_else(|| parse_error(token.pos, "Unknown search qualifier"))?;
            if matches!(self.current.kind, TokenKind::LParen) {
                self.bump()?;
                let value = self.or(depth + 1, Some((field, false)))?;
                if !matches!(self.current.kind, TokenKind::RParen) {
                    return Err(parse_error(self.current.pos, "Expected )"));
                }
                self.bump()?;
                return Ok(value);
            }
            let value = self.bump()?;
            let TokenKind::Word(text, glob) = value.kind else {
                return Err(parse_error(value.pos, "Expected text after qualifier"));
            };
            self.term()?;
            return Ok(Expr::Text(field, Pattern { text, glob }));
        }
        if matches!(self.current.kind, TokenKind::Exact) && inherited.is_none() {
            let field = parse_field(&text)
                .ok_or_else(|| parse_error(token.pos, "Unknown search qualifier"))?;
            self.bump()?;
            if matches!(self.current.kind, TokenKind::LParen) {
                self.bump()?;
                let value = self.or(depth + 1, Some((field, true)))?;
                if !matches!(self.current.kind, TokenKind::RParen) {
                    return Err(parse_error(self.current.pos, "Expected )"));
                }
                self.bump()?;
                return Ok(value);
            }
            let value = self.bump()?;
            let TokenKind::Word(text, _) = value.kind else {
                return Err(parse_error(
                    value.pos,
                    "Expected text after exact-match operator",
                ));
            };
            self.term()?;
            return Ok(Expr::Exact(field, Pattern { text, glob: false }));
        }
        self.term()?;
        let (field, exact) = inherited.unwrap_or((Field::Any, false));
        let pattern = Pattern {
            text,
            glob: glob && !exact,
        };
        Ok(if exact {
            Expr::Exact(field, pattern)
        } else {
            Expr::Text(field, pattern)
        })
    }
    fn term(&mut self) -> Result<(), Error> {
        self.terms += 1;
        if self.terms > MAX_TERMS {
            Err(Error("Search query exceeds 512 terms".into()))
        } else {
            Ok(())
        }
    }
}
fn parse_field(value: &str) -> Option<Field> {
    match value.to_ascii_lowercase().as_str() {
        "name" => Some(Field::Name),
        "path" => Some(Field::Path),
        "type" => Some(Field::Type),
        "class" => Some(Field::Class),
        "owner" => Some(Field::Owner),
        "ancestor" => Some(Field::Ancestor),
        "value" => Some(Field::Value),
        _ => None,
    }
}
fn parse_relation(value: &str) -> Option<Relation> {
    match value.to_ascii_lowercase().as_str() {
        "child" => Some(Relation::Child),
        "descendant" => Some(Relation::Descendant),
        "parent" => Some(Relation::Parent),
        _ => None,
    }
}
fn is_local_expression(value: &Expr) -> bool {
    match value {
        Expr::Text(field, _) | Expr::Exact(field, _) => {
            matches!(
                field,
                Field::Name | Field::Type | Field::Class | Field::Value
            )
        }
        Expr::Number(_, _) => true,
        Expr::Relation(_, _, inner) | Expr::Not(inner) => is_local_expression(inner),
        Expr::And(values) | Expr::Or(values) => values.iter().all(is_local_expression),
    }
}
fn parse_error(pos: usize, message: &str) -> Error {
    Error(format!("Search syntax at character {}: {message}", pos + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(q: &str) -> Result<Expr, Error> {
        Parser::new(q)?.parse()
    }
    #[test]
    fn parser_accepts_groups_qualifiers_and_ranges() {
        parse("path:*inventory* ancestor:Item type:(int | float) value>=-1.5e2 value<100").unwrap();
        parse("(value:*iron* | value:\"copper > ore\") !path:*equipment*").unwrap();
        parse("name==worldId value==(Prison | RuinedTemple)").unwrap();
        parse("child:(name==worldId value==(Prison | RuinedTemple))").unwrap();
        parse("parent:(child:(name==id value==PalaceSewer))").unwrap();
    }
    #[test]
    fn parser_reports_bad_input() {
        assert!(parse("(foo | bar").unwrap_err().0.contains("Expected )"));
        assert!(parse("value>=nope").is_err());
        assert!(parse("descendant:(path:*inventory*)").is_err());
    }
    #[test]
    fn wildcard_is_whole_field() {
        assert!(glob("iron_ore", "iron*"));
        assert!(!glob("some_iron_ore", "iron*"));
        assert!(glob("iron1", "iron?"));
    }
    #[test]
    fn class_names_match_short_namespace_and_serialized_forms() {
        let value = "Game.Items.Item, Assembly-CSharp";
        for text in ["Item", "Game.Items.Item", value] {
            assert!(matches_class(
                std::iter::once(value),
                &Pattern {
                    text: text.into(),
                    glob: false,
                },
                false,
            ));
        }
    }
    #[test]
    fn exact_decimal_handles_large_integers() {
        let a = Decimal::parse("9223372036854775807").unwrap();
        let b = Decimal::parse("9223372036854775806").unwrap();
        assert_eq!(a.compare(&b), Ordering::Greater);
        assert_eq!(
            Decimal::parse("1.50").unwrap(),
            Decimal::parse("1.5").unwrap()
        );
    }
}
