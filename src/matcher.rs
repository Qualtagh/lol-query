use lol_html::html_content::Element;
use lol_html::{HandlerResult, LocalHandlerTypes, element};

use crate::HandlerEntry;

trait Predicate: for<'r, 't> Fn(&mut Element<'r, 't, LocalHandlerTypes>) -> bool + 'static {}
impl<F: for<'r, 't> Fn(&mut Element<'r, 't, LocalHandlerTypes>) -> bool + 'static> Predicate for F {}

trait Callback: for<'r, 't> FnMut(&mut Element<'r, 't, LocalHandlerTypes>) + 'static {}
impl<F: for<'r, 't> FnMut(&mut Element<'r, 't, LocalHandlerTypes>) + 'static> Callback for F {}

/// Builder for advanced CSS-selector + predicate element matching.
///
/// Call [`filter`](Self::filter) or [`css`](Self::css), then [`on_each`](Self::on_each),
/// then [`build`](Self::build).
///
/// # Example
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
/// let handlers = Matcher::new()
///     .filter(".item", |el| el.get_attribute("data-x").is_some())
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
/// rw.write(b"<div class=\"item\" id=\"a\" data-x=\"1\"></div><div class=\"item\" id=\"b\"></div>").unwrap();
/// rw.end().unwrap();
///
/// assert_eq!(*ids.borrow(), ["a"]);
/// ```
pub struct Matcher {
    selector: Option<String>,
    predicate: Option<Box<dyn Predicate>>,
    callback: Option<Box<dyn Callback>>,
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
        Matcher { selector: None, predicate: None, callback: None }
    }

    /// Selects elements matching `selector` that also satisfy `predicate`.
    pub fn filter(mut self, selector: impl Into<String>, predicate: impl Predicate) -> Self {
        assert!(self.callback.is_none(), "filter() must be called before on_each()");
        self.selector = Some(selector.into());
        self.predicate = Some(Box::new(predicate));
        self
    }

    /// Selects all elements matching `selector`.
    pub fn css(self, selector: impl Into<String>) -> Self {
        self.filter(selector, |_| true)
    }

    /// Registers `callback` to run for each element that passes the predicate.
    pub fn on_each(mut self, callback: impl Callback) -> Self {
        assert!(self.selector.is_some(), "on_each() requires filter() or css() to be called first");
        assert!(self.callback.is_none(), "on_each() can only be called once");
        self.callback = Some(Box::new(callback));
        self
    }

    /// Consumes the builder and returns [`HandlerEntry`]s for use with
    /// [`SettingsExt::add_handlers`](crate::SettingsExt::add_handlers).
    pub fn build(self) -> Vec<HandlerEntry<'static, 'static>> {
        let selector = self.selector.expect("build() requires filter() or css() to be called first");
        let predicate = self.predicate.unwrap();
        let mut callback = self.callback.expect("build() requires on_each() to be called first");
        vec![element!(selector.as_str(), move |el: &mut Element<'_, '_, LocalHandlerTypes>| -> HandlerResult {
            if !predicate(el) { return Ok(()) };
            callback(el);
            Ok(())
        })
        .into()]
    }
}

#[cfg(test)]
#[path = "matcher.test.rs"]
mod tests;
