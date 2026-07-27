use std::cell::RefCell;
use std::rc::Rc;

use lol_html::html_content::{Element, EndTag, TextChunk};
use lol_html::{HandlerResult, LocalHandlerTypes, element, text};

use crate::HandlerEntry;

type El<'r, 't> = Element<'r, 't, LocalHandlerTypes>;

trait Predicate: for<'r, 't> Fn(&mut El<'r, 't>) -> bool + 'static {}
impl<F: for<'r, 't> Fn(&mut El<'r, 't>) -> bool + 'static> Predicate for F {}

trait Callback: for<'r, 't> FnMut(&mut El<'r, 't>) + 'static {}
impl<F: for<'r, 't> FnMut(&mut El<'r, 't>) + 'static> Callback for F {}

trait TextCallback: FnMut(&mut TextChunk<'_>) + 'static {}
impl<F: FnMut(&mut TextChunk<'_>) + 'static> TextCallback for F {}

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
}

/// A [`Step`] compiled into indices into [`State`].
enum Test {
    Leaf(usize),
    Not(usize),
    Every(Vec<usize>),
    Any(Vec<usize>),
}

impl Test {
    /// Tells whether the element currently being handled satisfies this test.
    fn holds(&self, state: &State) -> bool {
        match self {
            Test::Leaf(leaf) => state.hits[*leaf],
            Test::Not(chain) => !state.matched[*chain],
            Test::Every(chains) => chains.iter().all(|&chain| state.matched[chain]),
            Test::Any(chains) => chains.iter().any(|&chain| state.matched[chain]),
        }
    }
}

/// A whole [`Matcher`] tree flattened into ancestry chains of [`Test`]s.
#[derive(Default)]
struct Program {
    /// Every chain is preceded by the chains it refers to, so the root chain comes last.
    chains: Vec<Vec<Test>>,
    /// The selector and the predicate behind each [`Test::Leaf`].
    leaves: Vec<(String, Option<Box<dyn Predicate>>)>,
    /// Whether some test has to be evaluated for elements that no selector matches.
    universal: bool,
}

impl Program {
    /// Compiles `steps` into a chain and returns its index.
    fn add(&mut self, steps: Vec<Step>) -> usize {
        let chain = steps.into_iter().map(|step| self.compile(step)).collect();
        self.chains.push(chain);
        self.chains.len() - 1
    }

    fn compile(&mut self, step: Step) -> Test {
        match step {
            Step::Filter(selector, predicate) => {
                self.leaves.push((selector, predicate));
                Test::Leaf(self.leaves.len() - 1)
            },
            Step::Not(matcher) => {
                self.universal = true;
                Test::Not(self.add(matcher.steps))
            },
            Step::Every(matchers) => Test::Every(matchers.into_iter().map(|matcher| self.add(matcher.steps)).collect()),
            Step::Any(matchers) => Test::Any(matchers.into_iter().map(|matcher| self.add(matcher.steps)).collect()),
        }
    }
}

/// Matching progress shared by all handlers built by one [`Matcher::build`] call.
struct State {
    /// Per leaf: whether its selector and predicate matched the element currently being handled.
    hits: Vec<bool>,
    /// Per chain: whether it matches the element currently being handled.
    matched: Vec<bool>,
    /// Per chain level: the number of still open elements matched at that level.
    open: Vec<Vec<u32>>,
    /// The number of still open elements that matched the root chain and own text callbacks.
    text_open: u32,
}

/// Validates a matcher passed to [`Matcher::not`], [`Matcher::every`] or [`Matcher::any`].
fn nested(matcher: Matcher) -> Matcher {
    assert!(!matcher.steps.is_empty(), "a nested matcher needs at least one selector");
    assert!(matcher.callback.is_none(), "a nested matcher must not have an on_each() callback");
    assert!(matcher.text_callback.is_none(), "a nested matcher must not have an on_text_chunk() callback");
    matcher
}

/// Builder for advanced element matching: CSS selectors, custom predicates, ancestry
/// chains and boolean combinators.
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
///         Matcher::new().any(vec![Matcher::new().css("a"), Matcher::new().css("img")]),
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
        Matcher { steps: vec![], callback: None, text_callback: None }
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

    /// Appends a link to the ancestry chain.
    fn step(mut self, step: Step) -> Self {
        assert!(self.callback.is_none() && self.text_callback.is_none(), "selectors must be added before on_each() or on_text_chunk()");
        self.steps.push(step);
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

    /// Consumes the builder and returns [`HandlerEntry`]s for use with
    /// [`SettingsExt::add_handlers`](crate::SettingsExt::add_handlers).
    pub fn build(self) -> Vec<HandlerEntry<'static, 'static>> {
        assert!(self.callback.is_some() || self.text_callback.is_some(), "build() requires on_each() or on_text_chunk()");
        assert!(!self.steps.is_empty(), "build() requires a selector to be added first");
        if self.steps.len() == 1 && matches!(&self.steps[0], Step::Filter(_, _)) {
            return self.build_single();
        }
        self.build_chain()
    }

    fn build_single(self) -> Vec<HandlerEntry<'static, 'static>> {
        let Matcher { steps, callback, text_callback } = self;
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
            return handlers;
        }
        let predicate = predicate.unwrap();
        if text_callback.is_none() {
            let mut callback = callback.expect("build() requires on_each() or on_text_chunk()");
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
        let mut text_callback = text_callback.expect("build() requires on_each() or on_text_chunk()");
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
        handlers.push(
            text!(selector.as_str(), move |chunk: &mut TextChunk<'_>| -> HandlerResult {
                if !*active.borrow() { return Ok(()) };
                text_callback(chunk);
                Ok(())
            })
            .into(),
        );
        handlers
    }

    fn build_chain(self) -> Vec<HandlerEntry<'static, 'static>> {
        let Matcher { steps, callback, text_callback } = self;
        let mut program = Program::default();
        let root = program.add(steps);
        let Program { chains, leaves, universal } = program;
        let shared = Rc::new(RefCell::new(State {
            hits: vec![false; leaves.len()],
            matched: vec![false; chains.len()],
            open: chains.iter().map(|chain| vec![0; chain.len()]).collect(),
            text_open: 0,
        }));

        // Elements are matched selector by selector, and the leaf hits are combined into
        // chain matches afterwards, hence the two rounds of handlers. Handlers run in
        // registration order, so the second round sees all hits of the current element.
        let union = leaves.iter().map(|(selector, _)| selector.as_str()).collect::<Vec<_>>().join(", ");
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

        // Without a not() there is nothing to match on an element that no selector hit.
        let scope = if universal { "*" } else { union.as_str() };
        let scope_for_text = scope.to_string();
        let has_text = text_callback.is_some();
        let shared_for_combine = shared.clone();
        let mut callback = callback;
        let combine = element!(scope, move |el: &mut El<'_, '_>| -> HandlerResult {
            let mut opened = vec![];
            let state = &mut *shared_for_combine.borrow_mut();
            state.matched.fill(false);
            for (index, chain) in chains.iter().enumerate() {
                for (level, test) in chain.iter().enumerate() {
                    // An element is never its own ancestor: levels are opened only once all of them are resolved.
                    if !test.holds(state) || level > 0 && state.open[index][level - 1] == 0 { continue };
                    if level + 1 == chain.len() {
                      state.matched[index] = true;
                    } else {
                      opened.push((index, level));
                    }
                }
            }
            // This handler is the last one to run for the element, so hits can be recycled here.
            state.hits.fill(false);
            if !opened.is_empty() && let Some(end_tag_handlers) = el.end_tag_handlers() {
                for &(index, level) in &opened {
                  state.open[index][level] += 1;
                }
                let shared = shared_for_combine.clone();
                end_tag_handlers.push(Box::new(move |_: &mut EndTag<'_>| {
                    let state = &mut *shared.borrow_mut();
                    for (index, level) in opened {
                      state.open[index][level] -= 1;
                    }
                    Ok(())
                }));
            }
            if state.matched[root] {
              if let Some(callback) = callback.as_mut() {
                  callback(el);
              }
              if has_text {
                state.text_open += 1;
                if let Some(end_tag_handlers) = el.end_tag_handlers() {
                    let shared = shared_for_combine.clone();
                    end_tag_handlers.push(Box::new(move |_: &mut EndTag<'_>| {
                        shared.borrow_mut().text_open -= 1;
                        Ok(())
                    }));
                }
              }
            }
            Ok(())
        });
        handlers.push(combine.into());
        if let Some(mut text_callback) = text_callback {
            let shared = shared.clone();
            handlers.push(
                text!(scope_for_text.as_str(), move |chunk: &mut TextChunk<'_>| -> HandlerResult {
                    if shared.borrow().text_open == 0 { return Ok(()) };
                    text_callback(chunk);
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
