use std::cell::RefCell;
use std::rc::Rc;

use lol_html::html_content::{Comment, Element, EndTag, TextChunk};
use lol_html::{HandlerResult, LocalHandlerTypes, comments, element, text};

use crate::HandlerEntry;
use crate::engine::{AggregatedTextCallback, Callback, CommentCallback, Engine, Predicate, Step, TextCallback};

type El<'r, 't> = Element<'r, 't, LocalHandlerTypes>;

/// Validates a matcher nested inside a selector or gap constraint and returns its steps.
fn nested(matcher: Matcher) -> Vec<Step> {
    assert!(!matcher.steps.is_empty(), "a nested matcher needs at least one selector");
    assert!(!matcher.steps.last().unwrap().is_gap(), "a nested matcher cannot end with a gap selector");
    assert!(matcher.callback.is_none(), "a nested matcher must not have an on_each() callback");
    assert!(matcher.text_callback.is_none(), "a nested matcher must not have an on_text_chunk() callback");
    assert!(matcher.aggregated_text_callback.is_none(), "a nested matcher must not have an on_text() callback");
    assert!(matcher.comment_callback.is_none(), "a nested matcher must not have an on_comment() callback");
    matcher.steps
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
                if !predicate(el) {
                    return Ok(());
                }
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
                if !ok {
                    return Ok(());
                }
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
                    if !*active.borrow() {
                        return Ok(());
                    }
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
                    if !*active.borrow() {
                        return Ok(());
                    }
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
        let mut engine = Engine::new();
        let root = engine.add_chain(steps);
        if let Some(callback) = callback {
            engine.on_match(root, callback);
        }
        if let Some(callback) = text_callback {
            engine.on_text_chunk(root, callback);
        }
        if let Some(callback) = aggregated_text_callback {
            engine.on_text(root, callback);
        }
        if let Some(callback) = comment_callback {
            engine.on_comment(root, callback);
        }
        engine.into_handlers()
    }
}

#[cfg(test)]
#[path = "matcher.test.rs"]
mod tests;
