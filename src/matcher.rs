use std::cell::RefCell;
use std::rc::Rc;

use lol_html::html_content::{Comment, Element, EndTag, TextChunk};
use lol_html::{HandlerResult, LocalHandlerTypes, comments, element, text};

use crate::HandlerEntry;
use crate::general_regex::{GenRegExp, Pattern};

type El<'r, 't> = Element<'r, 't, LocalHandlerTypes>;

trait Predicate: for<'r, 't> Fn(&mut El<'r, 't>) -> bool + 'static {}
impl<F: for<'r, 't> Fn(&mut El<'r, 't>) -> bool + 'static> Predicate for F {}

trait Callback: for<'r, 't> FnMut(&mut El<'r, 't>) + 'static {}
impl<F: for<'r, 't> FnMut(&mut El<'r, 't>) + 'static> Callback for F {}

trait TextCallback: FnMut(&mut TextChunk<'_>) + 'static {}
impl<F: FnMut(&mut TextChunk<'_>) + 'static> TextCallback for F {}

trait AggregatedTextCallback: FnMut(&str) + 'static {}
impl<F: FnMut(&str) + 'static> AggregatedTextCallback for F {}

trait CommentCallback: FnMut(&mut Comment<'_>) + 'static {}
impl<F: FnMut(&mut Comment<'_>) + 'static> CommentCallback for F {}

/// One link of a [`Matcher`] ancestry chain.
enum Step {
    /// A CSS selector, optionally paired with a predicate on the candidate element.
    /// [`None`] means any element matching the selector, as with [`Matcher::css`].
    Filter(String, Option<Box<dyn Predicate>>),
    /// Elements that the nested matcher does not match.
    Not(Matcher),
    /// Elements that all of the nested matchers match.
    Every(Vec<Matcher>),
    /// Elements that at least one of the nested matchers matches.
    Any(Vec<Matcher>),
    /// A gap containing no element matched by the nested matcher.
    GapWithout(Matcher),
    /// A gap containing matches for every nested matcher, in any order.
    GapWithEvery(Vec<Matcher>),
    /// A gap containing an element matched by at least one nested matcher.
    GapWithAny(Vec<Matcher>),
    /// A zero-length gap: the next selector must match a direct child.
    Direct,
}

impl Step {
    fn is_gap(&self) -> bool {
        matches!(self, Step::GapWithout(_) | Step::GapWithEvery(_) | Step::GapWithAny(_) | Step::Direct)
    }

    fn is_element(&self) -> bool {
        matches!(self, Step::Filter(_, _) | Step::Not(_) | Step::Every(_) | Step::Any(_))
    }
}

/// A [`Step`] compiled into indices into [`State`].
enum Test {
    Leaf(usize),
    Chain(usize),
    Not(usize),
    Every(Vec<usize>),
    Any(Vec<usize>),
}

impl Test {
    /// Tells whether the element currently being handled satisfies this test.
    fn holds(&self, state: &State) -> bool {
        match self {
            Test::Leaf(leaf) => state.hits[*leaf],
            Test::Chain(chain) => state.matched[*chain],
            Test::Not(chain) => !state.matched[*chain],
            Test::Every(chains) => chains.iter().all(|&chain| state.matched[chain]),
            Test::Any(chains) => chains.iter().any(|&chain| state.matched[chain]),
        }
    }
}

/// One matcher chain compiled into tests and a bitset regular expression.
struct CompiledChain {
    tests: Vec<Test>,
    regexp: GenRegExp,
}

impl CompiledChain {
    fn symbol(&self, state: &State) -> Vec<u64> {
        let mut symbol = vec![0; self.tests.len().div_ceil(u64::BITS as usize)];
        for (bit, test) in self.tests.iter().enumerate() {
            if !test.holds(state) {
                continue;
            }
            symbol[bit / u64::BITS as usize] |= 1 << (bit % u64::BITS as usize);
        }
        symbol
    }
}

/// A whole [`Matcher`] tree flattened into dependency-ordered ancestry chains.
#[derive(Default)]
struct Program {
    /// Every chain is preceded by the chains it refers to, so the root chain comes last.
    chains: Vec<CompiledChain>,
    /// The selector and the predicate behind each [`Test::Leaf`].
    leaves: Vec<(String, Option<Box<dyn Predicate>>)>,
}

impl Program {
    /// Compiles `steps` into a chain and returns its index.
    fn add(&mut self, steps: Vec<Step>) -> usize {
        let mut tests = vec![];
        let mut patterns = vec![];
        let mut has_element = false;
        let mut has_gap = false;
        for step in steps {
            if step.is_gap() {
                patterns.push(self.compile_gap(step, &mut tests));
                has_gap = true;
                continue;
            }
            if !has_gap {
                patterns.push(Pattern::universal());
            }
            let test = self.compile(step);
            let bit = tests.len();
            tests.push(test);
            patterns.push(Pattern::bit(bit));
            has_element = true;
            has_gap = false;
        }
        assert!(has_element, "a matcher needs at least one selector");
        assert!(!has_gap, "a gap selector must be followed by an element selector");

        let expression = Pattern::sequence(patterns);
        self.chains.push(CompiledChain { tests, regexp: GenRegExp::new(expression) });
        self.chains.len() - 1
    }

    fn compile(&mut self, step: Step) -> Test {
        match step {
            Step::Filter(selector, predicate) => {
                self.leaves.push((selector, predicate));
                Test::Leaf(self.leaves.len() - 1)
            },
            Step::Not(matcher) => Test::Not(self.add(matcher.steps)),
            Step::Every(matchers) => Test::Every(matchers.into_iter().map(|matcher| self.add(matcher.steps)).collect()),
            Step::Any(matchers) => Test::Any(matchers.into_iter().map(|matcher| self.add(matcher.steps)).collect()),
            Step::GapWithout(_) | Step::GapWithEvery(_) | Step::GapWithAny(_) | Step::Direct => unreachable!(),
        }
    }

    fn compile_gap(&mut self, step: Step, tests: &mut Vec<Test>) -> Pattern {
        match step {
            Step::Direct => Pattern::epsilon(),
            Step::GapWithout(matcher) => {
                let bit = self.add_nested_test(matcher, tests);
                Pattern::not_bit(bit).repeat()
            },
            Step::GapWithEvery(matchers) => {
                let requirements = matchers
                    .into_iter()
                    .map(|matcher| {
                        let bit = self.add_nested_test(matcher, tests);
                        Pattern::sequence(vec![Pattern::universal(), Pattern::bit(bit), Pattern::universal()])
                    })
                    .collect();
                Pattern::intersection(requirements)
            },
            Step::GapWithAny(matchers) => {
                let alternatives = matchers
                    .into_iter()
                    .map(|matcher| {
                        let bit = self.add_nested_test(matcher, tests);
                        Pattern::bit(bit)
                    })
                    .collect();
                Pattern::sequence(vec![Pattern::universal(), Pattern::choice(alternatives), Pattern::universal()])
            },
            Step::Filter(_, _) | Step::Not(_) | Step::Every(_) | Step::Any(_) => unreachable!(),
        }
    }

    fn add_nested_test(&mut self, matcher: Matcher, tests: &mut Vec<Test>) -> usize {
        tests.push(Test::Chain(self.add(matcher.steps)));
        tests.len() - 1
    }
}

/// Matching progress shared by all handlers built by one [`Matcher::build`] call.
struct State {
    /// Per leaf: whether its selector and predicate matched the element currently being handled.
    hits: Vec<bool>,
    /// Per chain: whether it matches the element currently being handled.
    matched: Vec<bool>,
    /// Per chain: the DFA state after consuming the currently open ancestry.
    regexp_states: Vec<usize>,
    /// The number of still open elements that matched the root chain and own text callbacks.
    text_open: u32,
    /// Text chunks seen under currently open [`Matcher::on_text`] matches; each chunk is stored once.
    text_chunks: Vec<String>,
    /// For each open [`Matcher::on_text`] match (outermost last): start index into [`Self::text_chunks`].
    text_starts: Vec<usize>,
    /// The number of still open elements that matched the root chain and own comment callbacks.
    comment_open: u32,
}

/// Validates a matcher nested inside a selector or gap constraint.
fn nested(matcher: Matcher) -> Matcher {
    assert!(!matcher.steps.is_empty(), "a nested matcher needs at least one selector");
    assert!(!matcher.steps.last().unwrap().is_gap(), "a nested matcher cannot end with a gap selector");
    assert!(matcher.callback.is_none(), "a nested matcher must not have an on_each() callback");
    assert!(matcher.text_callback.is_none(), "a nested matcher must not have an on_text_chunk() callback");
    assert!(matcher.aggregated_text_callback.is_none(), "a nested matcher must not have an on_text() callback");
    assert!(matcher.comment_callback.is_none(), "a nested matcher must not have an on_comment() callback");
    matcher
}

/// Builder for advanced element matching: CSS selectors, custom predicates, ancestry
/// chains, constrained gaps and boolean combinators.
///
/// ## Element-level selectors
///
/// - [`filter`](Self::filter) — selector plus custom predicate
/// - [`css`](Self::css) — CSS selector
/// - [`not`](Self::not) — negated nested matcher
/// - [`every`](Self::every) — all nested matchers
/// - [`any`](Self::any) — any nested matcher
///
/// ## Gap-level selectors
///
/// - [`gap_without`](Self::gap_without) — gap with no nested match
/// - [`gap_with_every`](Self::gap_with_every) — gap with every nested matcher
/// - [`gap_with_any`](Self::gap_with_any) — gap with any nested matcher
/// - [`direct`](Self::direct) — direct child (`>` combinator)
///
/// ## Callbacks
///
/// - [`on_each`](Self::on_each) — per matched element
/// - [`on_text_chunk`](Self::on_text_chunk) — per text chunk
/// - [`on_text`](Self::on_text) — aggregated element text
/// - [`on_comment`](Self::on_comment) — per HTML comment
///
/// ## Other methods
///
/// - [`build`](Self::build) — get handler entries for HtmlRewriter
///
/// # Example: ancestry and combinators
///
/// ```rust
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// use lol_query::{Matcher, SettingsExt};
/// use lol_html::{HtmlRewriter, Settings};
///
/// let ids: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
/// let ids2 = ids.clone();
///
/// // Links and images inside a `.root` section, except the ones marked as hidden.
/// let handlers = Matcher::new()
///     .css(".root")
///     .every(vec![
///         Matcher::new().any(vec![
///             Matcher::new().css("a"),
///             Matcher::new().css("img"),
///         ]),
///         Matcher::new().not(Matcher::new().css(".hidden")),
///     ])
///     .on_each(move |el| {
///         ids2.borrow_mut().push(el.get_attribute("id").unwrap_or_default());
///     })
///     .build();
///
/// let mut out = vec![];
/// let mut rw = HtmlRewriter::new(
///     Settings::new().add_handlers(handlers),
///     |c: &[u8]| out.extend_from_slice(c),
/// );
/// rw.write(b"<div class=\"root\"><a id=\"a\"></a><img id=\"b\"><i class=\"hidden\" id=\"c\"></i></div><a id=\"d\"></a>").unwrap();
/// rw.end().unwrap();
///
/// assert_eq!(*ids.borrow(), ["a", "b"]);
/// ```
pub struct Matcher {
    steps: Vec<Step>,
    callback: Option<Box<dyn Callback>>,
    text_callback: Option<Box<dyn TextCallback>>,
    aggregated_text_callback: Option<Box<dyn AggregatedTextCallback>>,
    comment_callback: Option<Box<dyn CommentCallback>>,
}

impl Default for Matcher {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(private_bounds)]
impl Matcher {
    /// Creates a new [`Matcher`] builder.
    pub fn new() -> Self {
        Matcher { steps: vec![], callback: None, text_callback: None, aggregated_text_callback: None, comment_callback: None }
    }

    /// Selects elements matching `selector` that also satisfy `predicate`.
    pub fn filter(self, selector: impl Into<String>, predicate: impl Predicate) -> Self {
        self.step(Step::Filter(selector.into(), Some(Box::new(predicate))))
    }

    /// Selects all elements matching `selector`.
    pub fn css(self, selector: impl Into<String>) -> Self {
        self.step(Step::Filter(selector.into(), None))
    }

    /// Selects elements that `matcher` does not match, like the `:not()` pseudo-class.
    pub fn not(self, matcher: Matcher) -> Self {
        self.step(Step::Not(nested(matcher)))
    }

    /// Selects elements that all of the `matchers` match.
    pub fn every(self, matchers: Vec<Matcher>) -> Self {
        assert!(!matchers.is_empty(), "every() requires at least one matcher");
        self.step(Step::Every(matchers.into_iter().map(nested).collect()))
    }

    /// Selects elements that at least one of the `matchers` matches.
    pub fn any(self, matchers: Vec<Matcher>) -> Self {
        assert!(!matchers.is_empty(), "any() requires at least one matcher");
        self.step(Step::Any(matchers.into_iter().map(nested).collect()))
    }

    /// Constrains the gap before the next selector to contain no element
    /// matched by `matcher`. The gap may be empty.
    pub fn gap_without(self, matcher: Matcher) -> Self {
        self.gap(Step::GapWithout(nested(matcher)))
    }

    /// Constrains the gap before the next selector to contain at least one
    /// match for every nested matcher, in any order.
    pub fn gap_with_every(self, matchers: Vec<Matcher>) -> Self {
        assert!(!matchers.is_empty(), "gap_with_every() requires at least one matcher");
        self.gap(Step::GapWithEvery(matchers.into_iter().map(nested).collect()))
    }

    /// Constrains the gap before the next selector to contain at least one
    /// match for any of the nested matchers.
    pub fn gap_with_any(self, matchers: Vec<Matcher>) -> Self {
        assert!(!matchers.is_empty(), "gap_with_any() requires at least one matcher");
        self.gap(Step::GapWithAny(matchers.into_iter().map(nested).collect()))
    }

    /// Requires the next selector to match a direct child, like the `>` CSS combinator.
    pub fn direct(self) -> Self {
        assert!(!self.steps.is_empty(), "direct() cannot be the first selector");
        assert!(self.steps.last().is_some_and(Step::is_element), "direct() must follow an element selector");
        self.gap(Step::Direct)
    }

    /// Appends a link to the ancestry chain.
    fn step(mut self, step: Step) -> Self {
        assert!(
            self.callback.is_none() && self.text_callback.is_none() && self.aggregated_text_callback.is_none() && self.comment_callback.is_none(),
            "selectors must be added before callbacks"
        );
        self.steps.push(step);
        self
    }

    /// Appends a constrained gap to the ancestry chain.
    fn gap(mut self, gap: Step) -> Self {
        assert!(
            self.callback.is_none() && self.text_callback.is_none() && self.aggregated_text_callback.is_none() && self.comment_callback.is_none(),
            "selectors must be added before callbacks"
        );
        assert!(!matches!(self.steps.last(), Some(Step::Direct)), "direct() must be followed by an element selector");
        self.steps.push(gap);
        self
    }

    /// Registers `callback` to run once for each element matched by the whole chain,
    /// like the [`element!`](lol_html::element) macro.
    pub fn on_each(mut self, callback: impl Callback) -> Self {
        assert!(!self.steps.is_empty(), "on_each() requires a selector to be added first");
        assert!(self.callback.is_none(), "on_each() can only be called once");
        self.callback = Some(Box::new(callback));
        self
    }

    /// Registers `callback` to run for each text chunk inside elements matched by the whole chain,
    /// like the [`text!`](lol_html::text) macro.
    pub fn on_text_chunk(mut self, callback: impl TextCallback) -> Self {
        assert!(!self.steps.is_empty(), "on_text_chunk() requires a selector to be added first");
        assert!(self.text_callback.is_none(), "on_text_chunk() can only be called once");
        self.text_callback = Some(Box::new(callback));
        self
    }

    /// Registers `callback` to run once per element matched by the whole chain with the combined
    /// text of that element and all its descendants, like jQuery's [`.text()`](https://api.jquery.com/text/).
    pub fn on_text(mut self, callback: impl AggregatedTextCallback) -> Self {
        assert!(!self.steps.is_empty(), "on_text() requires a selector to be added first");
        assert!(self.aggregated_text_callback.is_none(), "on_text() can only be called once");
        self.aggregated_text_callback = Some(Box::new(callback));
        self
    }

    /// Registers `callback` to run for each HTML comment inside elements matched by the whole chain,
    /// like the [`comments!`](lol_html::comments) macro.
    pub fn on_comment(mut self, callback: impl CommentCallback) -> Self {
        assert!(!self.steps.is_empty(), "on_comment() requires a selector to be added first");
        assert!(self.comment_callback.is_none(), "on_comment() can only be called once");
        self.comment_callback = Some(Box::new(callback));
        self
    }

    /// Consumes the builder and returns [`HandlerEntry`]s for use with
    /// [`SettingsExt::add_handlers`](crate::SettingsExt::add_handlers).
    pub fn build(self) -> Vec<HandlerEntry<'static, 'static>> {
        assert!(
            self.callback.is_some() || self.text_callback.is_some() || self.aggregated_text_callback.is_some() || self.comment_callback.is_some(),
            "build() requires on_each(), on_text_chunk(), on_text() or on_comment()"
        );
        assert!(!self.steps.is_empty(), "build() requires a selector to be added first");
        assert!(!self.steps.last().unwrap().is_gap(), "a gap selector cannot be final in a chain");
        if self.aggregated_text_callback.is_none() && self.steps.len() == 1 && matches!(&self.steps[0], Step::Filter(_, _)) {
            return self.build_single();
        }
        self.build_chain()
    }

    fn build_single(self) -> Vec<HandlerEntry<'static, 'static>> {
        let Matcher { steps, callback, text_callback, comment_callback, aggregated_text_callback: _ } = self;
        let Step::Filter(selector, predicate) = steps.into_iter().next().unwrap() else { unreachable!() };
        if predicate.is_none() {
            let mut handlers = vec![];
            if let Some(mut callback) = callback {
                handlers.push(
                    element!(selector.as_str(), move |el: &mut El<'_, '_>| -> HandlerResult {
                        callback(el);
                        Ok(())
                    })
                    .into(),
                );
            }
            if let Some(mut text_callback) = text_callback {
                handlers.push(
                    text!(selector.as_str(), move |chunk: &mut TextChunk<'_>| -> HandlerResult {
                        text_callback(chunk);
                        Ok(())
                    })
                    .into(),
                );
            }
            if let Some(mut comment_callback) = comment_callback {
                handlers.push(
                    comments!(selector.as_str(), move |comment: &mut Comment<'_>| -> HandlerResult {
                        comment_callback(comment);
                        Ok(())
                    })
                    .into(),
                );
            }
            return handlers;
        }
        let predicate = predicate.unwrap();
        if text_callback.is_none() && comment_callback.is_none() {
            let mut callback = callback.expect("build() requires on_each(), on_text_chunk() or on_comment()");
            return vec![element!(selector.as_str(), move |el: &mut El<'_, '_>| -> HandlerResult {
                if !predicate(el) { return Ok(()) };
                callback(el);
                Ok(())
            })
            .into()];
        }
        let active = Rc::new(RefCell::new(false));
        let mut handlers = vec![];
        let active_for_el = active.clone();
        let mut callback = callback;
        handlers.push(
            element!(selector.as_str(), move |el: &mut El<'_, '_>| -> HandlerResult {
                let ok = predicate(el);
                *active_for_el.borrow_mut() = ok;
                if !ok { return Ok(()) };
                if let Some(callback) = callback.as_mut() {
                    callback(el);
                }
                if let Some(end_tag_handlers) = el.end_tag_handlers() {
                    let active = active_for_el.clone();
                    end_tag_handlers.push(Box::new(move |_: &mut EndTag<'_>| {
                        *active.borrow_mut() = false;
                        Ok(())
                    }));
                }
                Ok(())
            })
            .into(),
        );
        if let Some(mut text_callback) = text_callback {
            let active = active.clone();
            handlers.push(
                text!(selector.as_str(), move |chunk: &mut TextChunk<'_>| -> HandlerResult {
                    if !*active.borrow() { return Ok(()) };
                    text_callback(chunk);
                    Ok(())
                })
                .into(),
            );
        }
        if let Some(mut comment_callback) = comment_callback {
            let active = active.clone();
            handlers.push(
                comments!(selector.as_str(), move |comment: &mut Comment<'_>| -> HandlerResult {
                    if !*active.borrow() { return Ok(()) };
                    comment_callback(comment);
                    Ok(())
                })
                .into(),
            );
        }
        handlers
    }

    fn build_chain(self) -> Vec<HandlerEntry<'static, 'static>> {
        let Matcher { steps, callback, text_callback, aggregated_text_callback, comment_callback } = self;
        let mut program = Program::default();
        let root = program.add(steps);
        let Program { mut chains, leaves } = program;
        let shared = Rc::new(RefCell::new(State {
            hits: vec![false; leaves.len()],
            matched: vec![false; chains.len()],
            regexp_states: chains.iter().map(|chain| chain.regexp.start_state()).collect(),
            text_open: 0,
            text_chunks: vec![],
            text_starts: vec![],
            comment_open: 0,
        }));

        // Elements are matched selector by selector, and the leaf hits are combined into
        // chain matches afterwards, hence the two rounds of handlers. Handlers run in
        // registration order, so the second round sees all hits of the current element.
        let mut handlers: Vec<HandlerEntry<'static, 'static>> = leaves
            .into_iter()
            .enumerate()
            .map(|(leaf, (selector, predicate))| {
                let shared = shared.clone();
                element!(selector.as_str(), move |el: &mut El<'_, '_>| -> HandlerResult {
                    if let Some(predicate) = &predicate && !predicate(el) { return Ok(()) };
                    shared.borrow_mut().hits[leaf] = true;
                    Ok(())
                })
                .into()
            })
            .collect();

        let has_text_chunk = text_callback.is_some();
        let has_aggregated_text = aggregated_text_callback.is_some();
        let track_text = has_text_chunk || has_aggregated_text;
        let has_comment = comment_callback.is_some();
        let shared_for_combine = shared.clone();
        let mut callback = callback;
        let aggregated_text_callback = aggregated_text_callback.map(|callback| Rc::new(RefCell::new(callback)));
        let combine = element!("*", move |el: &mut El<'_, '_>| -> HandlerResult {
            let state = &mut *shared_for_combine.borrow_mut();
            let parent_states = state.regexp_states.clone();
            let mut child_states = Vec::with_capacity(chains.len());
            state.matched.fill(false);
            for (index, chain) in chains.iter_mut().enumerate() {
                let symbol = chain.symbol(state);
                let next = chain.regexp.transition(parent_states[index], &symbol);
                state.matched[index] = chain.regexp.is_match(next);
                child_states.push(next);
            }
            // This handler is the last one to run for the element, so hits can be recycled here.
            state.hits.fill(false);
            if let Some(end_tag_handlers) = el.end_tag_handlers() {
                state.regexp_states = child_states;
                let shared = shared_for_combine.clone();
                end_tag_handlers.push(Box::new(move |_: &mut EndTag<'_>| {
                    shared.borrow_mut().regexp_states = parent_states;
                    Ok(())
                }));
            }
            if state.matched[root] {
              if let Some(callback) = callback.as_mut() {
                  callback(el);
              }
              if track_text || has_comment {
                if track_text {
                  state.text_open += 1;
                  if has_aggregated_text {
                    state.text_starts.push(state.text_chunks.len());
                  }
                }
                if has_comment {
                  state.comment_open += 1;
                }
                if let Some(end_tag_handlers) = el.end_tag_handlers() {
                    let shared = shared_for_combine.clone();
                    let aggregated_text_callback = aggregated_text_callback.clone();
                    end_tag_handlers.push(Box::new(move |_: &mut EndTag<'_>| {
                        let state = &mut *shared.borrow_mut();
                        if track_text {
                          if has_aggregated_text {
                            let start = state.text_starts.pop().unwrap();
                            let len = state.text_chunks[start..].iter().map(String::len).sum();
                            let mut text = String::with_capacity(len);
                            for chunk in &state.text_chunks[start..] {
                              text.push_str(chunk);
                            }
                            if state.text_starts.is_empty() {
                              state.text_chunks.clear();
                            }
                            aggregated_text_callback.as_ref().unwrap().borrow_mut()(&text);
                          }
                          state.text_open -= 1;
                        }
                        if has_comment {
                          state.comment_open -= 1;
                        }
                        Ok(())
                    }));
                }
              }
            }
            Ok(())
        });
        handlers.push(combine.into());
        if track_text {
            let shared = shared.clone();
            let mut text_callback = text_callback;
            handlers.push(
                text!("*", move |chunk: &mut TextChunk<'_>| -> HandlerResult {
                    if shared.borrow().text_open == 0 { return Ok(()) };
                    if let Some(text_callback) = text_callback.as_mut() {
                        text_callback(chunk);
                    }
                    if has_aggregated_text {
                        shared.borrow_mut().text_chunks.push(chunk.as_str().to_owned());
                    }
                    Ok(())
                })
                .into(),
            );
        }
        if let Some(mut comment_callback) = comment_callback {
            let shared = shared.clone();
            handlers.push(
                comments!("*", move |comment: &mut Comment<'_>| -> HandlerResult {
                    if shared.borrow().comment_open == 0 { return Ok(()) };
                    comment_callback(comment);
                    Ok(())
                })
                .into(),
            );
        }
        handlers
    }
}

#[cfg(test)]
#[path = "matcher.test.rs"]
mod tests;
