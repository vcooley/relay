use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use failure::Fail;
use regex::Regex;
use smallvec::SmallVec;

use crate::processor::{ProcessValue, SelectorPathItem, SelectorSpec};
use crate::types::Annotated;

/// Error for unknown value types.
#[derive(Debug, Fail)]
#[fail(display = "unknown value type")]
pub struct UnknownValueTypeError;

/// The (simplified) type of a value.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum ValueType {
    String,
    Number,
    Boolean,
    DateTime,
    Array,
    Object,
    Event,
    Exception,
    Stacktrace,
    Frame,
    Request,
    User,
    LogEntry,
    Message,
    Thread,
    Breadcrumb,
    Span,
    ClientSdkInfo,
}

impl ValueType {
    pub fn for_field<T: ProcessValue>(field: &Annotated<T>) -> Option<Self> {
        field.value().and_then(ProcessValue::value_type)
    }

    pub fn name(self) -> &'static str {
        match self {
            ValueType::String => "string",
            ValueType::Number => "number",
            ValueType::Boolean => "boolean",
            ValueType::DateTime => "datetime",
            ValueType::Array => "array",
            ValueType::Object => "object",
            ValueType::Event => "event",
            ValueType::Exception => "error",
            ValueType::Stacktrace => "stack",
            ValueType::Frame => "frame",
            ValueType::Request => "http",
            ValueType::User => "user",
            ValueType::LogEntry => "logentry",
            ValueType::Message => "message",
            ValueType::Thread => "thread",
            ValueType::Breadcrumb => "breadcrumb",
            ValueType::Span => "span",
            ValueType::ClientSdkInfo => "sdk",
        }
    }
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for ValueType {
    type Err = UnknownValueTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "string" => ValueType::String,
            "number" => ValueType::Number,
            "bool" | "boolean" => ValueType::Boolean,
            "datetime" => ValueType::DateTime,
            "array" | "list" => ValueType::Array,
            "object" => ValueType::Object,
            "event" => ValueType::Event,
            "exception" | "error" => ValueType::Exception,
            "stacktrace" | "stack" => ValueType::Stacktrace,
            "frame" => ValueType::Frame,
            "request" | "http" => ValueType::Request,
            "user" => ValueType::User,
            "logentry" => ValueType::LogEntry,
            "message" => ValueType::Message,
            "thread" => ValueType::Thread,
            "breadcrumb" => ValueType::Breadcrumb,
            "sdk" => ValueType::ClientSdkInfo,
            _ => return Err(UnknownValueTypeError),
        })
    }
}

/// The maximum length of a field.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum MaxChars {
    Hash,
    EnumLike,
    Summary,
    Message,
    Symbol,
    Path,
    ShortPath,
    Logger,
    Email,
    Culprit,
    TagKey,
    TagValue,
    Environment,
    Hard(usize),
    Soft(usize),
}

impl MaxChars {
    /// The cap in number of unicode characters.
    pub fn limit(self) -> usize {
        match self {
            MaxChars::Hash => 128,
            MaxChars::EnumLike => 128,
            MaxChars::Summary => 1024,
            MaxChars::Message => 8192,
            MaxChars::Symbol => 256,
            MaxChars::Path => 256,
            MaxChars::ShortPath => 128,
            // these are from constants.py or limits imposed by the database
            MaxChars::Logger => 64,
            MaxChars::Email => 75,
            MaxChars::Culprit => 200,
            MaxChars::TagKey => 32,
            MaxChars::TagValue => 200,
            MaxChars::Environment => 64,
            MaxChars::Soft(len) | MaxChars::Hard(len) => len,
        }
    }

    /// The number of extra characters permitted.
    pub fn allowance(self) -> usize {
        match self {
            MaxChars::Hash => 0,
            MaxChars::EnumLike => 0,
            MaxChars::Summary => 100,
            MaxChars::Message => 200,
            MaxChars::Symbol => 20,
            MaxChars::Path => 40,
            MaxChars::ShortPath => 20,
            MaxChars::Logger => 0,
            MaxChars::Email => 0,
            MaxChars::Culprit => 0,
            MaxChars::TagKey => 0,
            MaxChars::TagValue => 0,
            MaxChars::Environment => 0,
            MaxChars::Soft(_) => 10,
            MaxChars::Hard(_) => 0,
        }
    }
}

/// The maximum size of a databag.
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum BagSize {
    Small,
    Medium,
    Large,
    Larger,
    Massive,
}

impl BagSize {
    /// Maximum depth of the structure.
    pub fn max_depth(self) -> usize {
        match self {
            BagSize::Small => 3,
            BagSize::Medium => 5,
            BagSize::Large => 7,
            BagSize::Larger => 7,
            BagSize::Massive => 7,
        }
    }

    /// Maximum estimated JSON bytes.
    pub fn max_size(self) -> usize {
        match self {
            BagSize::Small => 1024,
            BagSize::Medium => 2048,
            BagSize::Large => 8192,
            BagSize::Larger => 16384,
            BagSize::Massive => 262_144,
        }
    }
}

/// Whether an attribute should be PII-strippable/should be subject to datascrubbers
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Pii {
    /// The field will be stripped by default
    True,
    /// The field cannot be stripped at all
    False,
    /// The field will only be stripped when addressed with a specific path selector, but generic
    /// selectors such as `$string` do not apply.
    Maybe,
}

/// Meta information about a field.
#[derive(Debug, Clone)]
pub struct FieldAttrs {
    /// Optionally the name of the field.
    pub name: Option<&'static str>,
    /// If the field is required.
    pub required: bool,
    /// If the field should be non-empty.
    pub nonempty: bool,
    /// Whether to trim whitespace from this string.
    pub trim_whitespace: bool,
    /// A regex to validate the (string) value against.
    pub match_regex: Option<Regex>,
    /// The maximum char length of this field.
    pub max_chars: Option<MaxChars>,
    /// The maximum bag size of this field.
    pub bag_size: Option<BagSize>,
    /// The type of PII on the field.
    pub pii: Pii,
    /// Whether additional properties should be retained during normalization.
    pub retain: bool,
}

lazy_static::lazy_static! {
    static ref DEFAULT_FIELD_ATTRS: FieldAttrs = FieldAttrs {
        name: None,
        required: false,
        nonempty: false,
        trim_whitespace: false,
        match_regex: None,
        max_chars: None,
        bag_size: None,
        pii: Pii::False,
        retain: false,
    };

    static ref PII_TRUE_FIELD_ATTRS: FieldAttrs = FieldAttrs {
        pii: Pii::True,
        ..DEFAULT_FIELD_ATTRS.clone()
    };

    static ref PII_MAYBE_FIELD_ATTRS: FieldAttrs = FieldAttrs {
        pii: Pii::Maybe,
        ..DEFAULT_FIELD_ATTRS.clone()
    };
}

impl Default for FieldAttrs {
    fn default() -> FieldAttrs {
        DEFAULT_FIELD_ATTRS.clone()
    }
}

#[derive(Debug, Clone, Eq, Ord, PartialOrd)]
enum PathItem<'a> {
    StaticKey(&'a str),
    Index(usize),
}

impl<'a> PartialEq for PathItem<'a> {
    fn eq(&self, other: &PathItem<'a>) -> bool {
        match *self {
            PathItem::StaticKey(ref s) => other.key() == Some(s),
            PathItem::Index(value) => other.index() == Some(value),
        }
    }
}

impl<'a> PathItem<'a> {
    /// Returns the key if there is one
    #[inline]
    pub fn key(&self) -> Option<&str> {
        match *self {
            PathItem::StaticKey(s) => Some(s),
            PathItem::Index(_) => None,
        }
    }

    /// Returns the index if there is one
    #[inline]
    pub fn index(&self) -> Option<usize> {
        match *self {
            PathItem::StaticKey(_) => None,
            PathItem::Index(idx) => Some(idx),
        }
    }
}

impl<'a> fmt::Display for PathItem<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            PathItem::StaticKey(s) => f.pad(s),
            PathItem::Index(val) => write!(f, "{}", val),
        }
    }
}

/// Processing state passed downwards during processing.
#[derive(Debug, Clone)]
pub struct ProcessingState<'a> {
    parent: Option<&'a ProcessingState<'a>>,
    path_item: Option<PathItem<'a>>,
    attrs: Option<Cow<'a, FieldAttrs>>,
    value_type: Option<ValueType>,
    depth: usize,
}

static ROOT_STATE: ProcessingState = ProcessingState {
    parent: None,
    path_item: None,
    attrs: None,
    value_type: None,
    depth: 0,
};

impl<'a> ProcessingState<'a> {
    /// Returns the root processing state.
    pub fn root() -> &'static ProcessingState<'static> {
        &ROOT_STATE
    }

    /// Creates a new root state.
    pub fn new_root(
        attrs: Option<Cow<'static, FieldAttrs>>,
        value_type: Option<ValueType>,
    ) -> ProcessingState<'static> {
        ProcessingState {
            parent: None,
            path_item: None,
            attrs,
            value_type,
            depth: 0,
        }
    }

    /// Derives a processing state by entering a static key.
    pub fn enter_static(
        &'a self,
        key: &'static str,
        attrs: Option<Cow<'static, FieldAttrs>>,
        value_type: Option<ValueType>,
    ) -> Self {
        ProcessingState {
            parent: Some(self),
            path_item: Some(PathItem::StaticKey(key)),
            attrs,
            value_type,
            depth: self.depth + 1,
        }
    }

    /// Derives a processing state by entering a borrowed key.
    pub fn enter_borrowed(
        &'a self,
        key: &'a str,
        attrs: Option<Cow<'a, FieldAttrs>>,
        value_type: Option<ValueType>,
    ) -> Self {
        ProcessingState {
            parent: Some(self),
            path_item: Some(PathItem::StaticKey(key)),
            attrs,
            value_type,
            depth: self.depth + 1,
        }
    }

    /// Derives a processing state by entering an index.
    pub fn enter_index(
        &'a self,
        idx: usize,
        attrs: Option<Cow<'a, FieldAttrs>>,
        value_type: Option<ValueType>,
    ) -> Self {
        ProcessingState {
            parent: Some(self),
            path_item: Some(PathItem::Index(idx)),
            attrs,
            value_type,
            depth: self.depth + 1,
        }
    }

    /// Derives a processing state without adding a path segment. Useful in newtype structs.
    pub fn enter_nothing(&'a self, attrs: Option<Cow<'a, FieldAttrs>>) -> Self {
        ProcessingState {
            attrs,
            path_item: None,
            parent: Some(self),
            ..self.clone()
        }
    }

    /// Returns the path in the processing state.
    pub fn path(&'a self) -> Path<'a> {
        Path(&self)
    }

    pub fn value_type(&self) -> Option<ValueType> {
        self.value_type
    }

    /// Returns the field attributes.
    pub fn attrs(&self) -> &FieldAttrs {
        match self.attrs {
            Some(ref cow) => &cow,
            None => &DEFAULT_FIELD_ATTRS,
        }
    }

    /// Derives the attrs for recursion.
    pub fn inner_attrs(&self) -> Option<Cow<'_, FieldAttrs>> {
        match self.attrs().pii {
            Pii::True => Some(Cow::Borrowed(&PII_TRUE_FIELD_ATTRS)),
            Pii::False => None,
            Pii::Maybe => Some(Cow::Borrowed(&PII_MAYBE_FIELD_ATTRS)),
        }
    }

    /// Iterates through this state and all its ancestors up the hierarchy.
    pub fn iter(&'a self) -> ProcessingStateIter<'a> {
        ProcessingStateIter {
            state: Some(self),
            size: self.depth,
        }
    }

    /// Return the depth (~ indentation level) of the currently processed value.
    pub fn depth(&'a self) -> usize {
        self.depth
    }

    /// Return whether the depth changed between parent and self.
    ///
    /// This is `false` when we entered a newtype struct.
    pub fn entered_anything(&'a self) -> bool {
        if let Some(ref parent) = self.parent {
            parent.depth() != self.depth()
        } else {
            true
        }
    }

    /// Returns the last path item if there is one. Skips over "dummy" path segments that exist
    /// because of newtypes.
    #[inline]
    fn path_item(&self) -> Option<&PathItem<'_>> {
        for state in self.iter() {
            if let Some(ref path_item) = state.path_item {
                return Some(path_item);
            }
        }
        None
    }
}

pub struct ProcessingStateIter<'a> {
    state: Option<&'a ProcessingState<'a>>,
    size: usize,
}

impl<'a> Iterator for ProcessingStateIter<'a> {
    type Item = &'a ProcessingState<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.state?;
        self.state = current.parent;
        Some(current)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size, Some(self.size))
    }
}

impl<'a> ExactSizeIterator for ProcessingStateIter<'a> {}

impl<'a> Default for ProcessingState<'a> {
    fn default() -> Self {
        ProcessingState::root().clone()
    }
}

/// Represents the path in a structure
#[derive(Debug)]
pub struct Path<'a>(&'a ProcessingState<'a>);

impl<'a> Path<'a> {
    /// Returns the current key if there is one
    #[inline]
    pub fn key(&self) -> Option<&str> {
        PathItem::key(self.0.path_item()?)
    }

    /// Returns the current index if there is one
    #[inline]
    pub fn index(&self) -> Option<usize> {
        PathItem::index(self.0.path_item()?)
    }

    /// Checks if a path matches given selector.
    pub fn matches_selector(&self, pii: Pii, selector: &SelectorSpec) -> bool {
        match *selector {
            SelectorSpec::Path(ref path) => {
                // fastest path: the selector is deeper than the current structure, or the field is
                // not supposed to be scrubbed.
                if path.len() > self.0.depth || pii == Pii::False {
                    return false;
                }

                // fast path: we do not have any deep matches
                let mut state_iter = self.0.iter().filter(|state| state.entered_anything());
                let mut selector_iter = path.iter().enumerate().rev();
                let mut depth_match = false;
                for state in &mut state_iter {
                    if !match selector_iter.next() {
                        Some((_, SelectorPathItem::DeepWildcard)) => {
                            if pii == Pii::Maybe {
                                return false;
                            }

                            depth_match = true;
                            break;
                        }
                        Some((i, ref path_item)) => path_item.matches_state(i, pii, state),
                        None => break,
                    } {
                        return false;
                    }
                }

                if !depth_match {
                    return true;
                }

                // slow path: we collect the remaining states and skip up to the first
                // match of the selector.
                let remaining_states = state_iter.collect::<SmallVec<[&ProcessingState<'_>; 16]>>();
                let mut selector_iter = selector_iter.rev().peekable();
                let (first_selector_path_pos, first_selector_path) = match selector_iter.next() {
                    Some(selector_path) => selector_path,
                    None => return !remaining_states.is_empty(),
                };
                let mut path_match_iterator = remaining_states.iter().rev().skip_while(|state| {
                    !first_selector_path.matches_state(first_selector_path_pos, pii, state)
                });
                if path_match_iterator.next().is_none() {
                    return false;
                }

                // then we check all remaining items and that nothing is left of the selector
                path_match_iterator
                    .zip(&mut selector_iter)
                    .all(|(state, (pos, selector_path))| {
                        selector_path.matches_state(pos, pii, state)
                    })
                    && selector_iter.next().is_none()
            }

            // Conjunction: At least one subselector needs to be specific if the selector needs to be
            // specific
            // All subselectors must match, generally (ignoring pii=maybe)
            SelectorSpec::And(ref xs) => xs.iter().all(|x| self.matches_selector(Pii::True, x)) && xs.iter().any(|x| self.matches_selector(pii, x)),

            // Disjunction: At least one subselector must match
            SelectorSpec::Or(ref xs) => xs.iter().any(|x| self.matches_selector(pii, x)),

            // Negation disallows pii=maybe.
            SelectorSpec::Not(ref x) => pii == Pii::True && !self.matches_selector(pii, x),
        }
    }
}

impl<'a> fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut items = Vec::with_capacity(self.0.depth);
        for state in self.0.iter() {
            if let Some(ref path_item) = state.path_item {
                items.push(path_item)
            }
        }

        for (idx, item) in items.into_iter().rev().enumerate() {
            if idx > 0 {
                write!(f, ".")?;
            }
            write!(f, "{}", item)?;
        }
        Ok(())
    }
}

#[allow(clippy::cognitive_complexity)]
#[test]
fn test_selector_matching() {
    use itertools::Itertools;

    macro_rules! assert_matches_raw {
        ($state:expr, $pii:expr, $selector:expr, $expected:expr) => {{
            assert!(
                $state
                    .path()
                    .matches_selector($pii, &$selector.parse().unwrap())
                    == $expected,
                format!(
                    "Matched {} against {} with pii={:?}, expected {}",
                    $selector,
                    $state.path(),
                    $pii,
                    $expected
                )
            );
        }};
    }

    macro_rules! assert_matches {
        ($state:expr, $($selector:expr,)*) => {{
            let state = &$state;
            $(
                assert_matches_raw!(state, Pii::True, $selector, true);
                assert_matches_raw!(state, Pii::Maybe, $selector, true);
                assert_matches_raw!(state, Pii::False, $selector, false);
            )*

            let joined = vec![$($selector),*].into_iter().join(" && ");
            assert_matches_raw!(state, Pii::True, &joined, true);
            assert_matches_raw!(state, Pii::Maybe, &joined, true);
            assert_matches_raw!(state, Pii::False, &joined, false);

            let joined = vec![$($selector),*].into_iter().join(" || ");
            assert_matches_raw!(state, Pii::True, &joined, true);
            assert_matches_raw!(state, Pii::Maybe, &joined, true);
            assert_matches_raw!(state, Pii::False, &joined, false);

            let joined = vec!["**", $($selector),*].into_iter().join(" || ");
            assert_matches_raw!(state, Pii::True, &joined, true);
            assert_matches_raw!(state, Pii::Maybe, &joined, true);
            assert_matches_raw!(state, Pii::False, &joined, false);
        }}
    }

    macro_rules! assert_matches_non_specific {
        ($state:expr, $($selector:expr,)*) => {{
            let state = &$state;
            $(
                assert_matches_raw!(state, Pii::True, $selector, true);
                assert_matches_raw!(state, Pii::Maybe, $selector, false);
                assert_matches_raw!(state, Pii::False, $selector, false);
            )*

            let joined = vec![$($selector),*].into_iter().join(" && ");
            assert_matches_raw!(state, Pii::True, &joined, true);
            assert_matches_raw!(state, Pii::Maybe, &joined, false);
            assert_matches_raw!(state, Pii::False, &joined, false);

            let joined = vec![$($selector),*].into_iter().join(" || ");
            assert_matches_raw!(state, Pii::True, &joined, true);
            assert_matches_raw!(state, Pii::Maybe, &joined, false);
            assert_matches_raw!(state, Pii::False, &joined, false);

            let joined = vec!["**", $($selector),*].into_iter().join(" || ");
            assert_matches_raw!(state, Pii::True, &joined, true);
            assert_matches_raw!(state, Pii::Maybe, &joined, false);
            assert_matches_raw!(state, Pii::False, &joined, false);
        }}
    }

    macro_rules! assert_not_matches {
        ($state:expr, $($selector:expr,)*) => {{
            let state = &$state;
            $(
                assert_matches_raw!(state, Pii::True, $selector, false);
                assert_matches_raw!(state, Pii::Maybe, $selector, false);
                assert_matches_raw!(state, Pii::False, $selector, false);
            )*
        }}
    }

    let event_state = ProcessingState::new_root(None, Some(ValueType::Event)); // .
    let user_state = event_state.enter_static("user", None, Some(ValueType::User)); // .user
    let extra_state = user_state.enter_static("extra", None, Some(ValueType::Object)); // .user.extra
    let foo_state = extra_state.enter_static("foo", None, Some(ValueType::Array)); // .user.extra.foo
    let zero_state = foo_state.enter_index(0, None, None); // .user.extra.foo.0

    assert_matches!(
        extra_state,
        "user.extra",  // this is an exact match to the state
        "$user.extra", // this is a match below a type
        "(** || user.*) && !(foo.bar.baz || a.b.c)",
    );

    assert_matches_non_specific!(
        extra_state,

        // known limitation: double-negations *could* be specific (I'd expect this as a user), but
        // right now we don't support it
        "!(!user.extra)", 
        "!(!$user.extra)",
    );

    assert_matches!(
        foo_state,
        "$user.extra.*", // this is a wildcard match into a type
    );

    assert_matches!(
        zero_state,
        "$user.extra.foo.*", // a wildcard match into an array
        "$user.extra.foo.0", // a direct match into an array
    );

    assert_matches_non_specific!(
        zero_state,
        // deep matches are wild
        "$user.extra.foo.**",
        "$user.extra.**",
        "$user.**",
        "$event.**",
        "$user.**.0",
        // types are anywhere
        "$user.$object.**.0",
        "(**.0 | absolutebogus)",
        "(~$object)",
        "($object.** & (~absolutebogus))",
        "($object.** & (~absolutebogus))",
    );

    assert_not_matches!(
        zero_state,
        "$user.extra.foo.1", // direct mismatch in an array
        // deep matches are wild
        "$user.extra.bar.**",
        "$user.**.1",
        "($object | absolutebogus)",
        "(~$object.**)",
        "($object | (**.0 & absolutebogus))",
    );

    assert_matches_non_specific!(
        foo_state,
        "($array & $object.*)",
        "(** & $object.*)",
        "**.$array",
    );

    assert_not_matches!(foo_state, "($object & $object.*)",);
}
