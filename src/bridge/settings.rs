use std::borrow::Cow;

use lol_html::errors::RewritingError;
use lol_html::html_content::BailOut;
use lol_html::{BailOutHandler, DocumentContentHandlers, ElementContentHandlers, Selector, Settings};

/// A unified handler entry for use with [`SettingsExt::add_handlers`].
///
/// Element and text handlers (from the `element!` and `text!` macros) convert
/// via [`From`]/[`.into()`](Into::into). Bail-out handlers are constructed with
/// [`HandlerEntry::bail_out`] because the `bail_out!` macro yields a raw closure,
/// not the boxed [`BailOutHandler`].
pub enum HandlerEntry<'h, 's> {
    /// An element-content handler paired with its CSS selector.
    Element(Cow<'s, Selector>, ElementContentHandlers<'h>),
    /// A document-level content handler (no selector).
    Document(DocumentContentHandlers<'h>),
    /// A graceful bail-out handler.
    BailOut(BailOutHandler<'h>),
}

impl<'h, 's> HandlerEntry<'h, 's> {
    /// Wraps a bail-out closure into a [`HandlerEntry`].
    ///
    /// Accepts the same closure type as `lol_html::bail_out!`, so the macro
    /// output can be passed directly:
    ///
    /// ```ignore
    /// HandlerEntry::bail_out(bail_out!(|_err, _bail_out| {}))
    /// ```
    pub fn bail_out<F>(handler: F) -> Self
    where
        F: FnMut(&RewritingError, &mut BailOut<'_>) + 'h,
    {
        Self::BailOut(Box::new(handler))
    }
}

/// Converts the tuple returned by `element!` / `text!` macros into a
/// [`HandlerEntry`].
impl<'h, 's> From<(Cow<'s, Selector>, ElementContentHandlers<'h>)> for HandlerEntry<'h, 's> {
    fn from((selector, handlers): (Cow<'s, Selector>, ElementContentHandlers<'h>)) -> Self {
        Self::Element(selector, handlers)
    }
}

/// Converts a [`DocumentContentHandlers`] value into a [`HandlerEntry`].
impl<'h, 's> From<DocumentContentHandlers<'h>> for HandlerEntry<'h, 's> {
    fn from(handlers: DocumentContentHandlers<'h>) -> Self {
        Self::Document(handlers)
    }
}

/// Extension trait that adds [`add_handlers`](SettingsExt::add_handlers) to
/// [`lol_html::Settings`].
///
/// Import this trait to bring the method into scope:
///
/// ```rust
/// use lol_query::SettingsExt;
/// use lol_html::{element, Settings};
///
/// let ids: Vec<String> = Vec::new();
/// # let _ = ids;
/// ```
pub trait SettingsExt<'h, 's> {
    /// Appends all entries in `handlers` to `self`, returning the updated
    /// [`Settings`].
    ///
    /// This is equivalent to calling [`Settings::append_element_content_handler`],
    /// [`Settings::append_document_content_handler`], or
    /// [`Settings::append_bail_out_handler`] once per entry, and is especially
    /// useful for passing the output of a [`Matcher`](crate::Matcher) in a
    /// single call.
    fn add_handlers(self, handlers: Vec<HandlerEntry<'h, 's>>) -> Self;
}

impl<'h, 's> SettingsExt<'h, 's> for Settings<'h, 's> {
    fn add_handlers(self, handlers: Vec<HandlerEntry<'h, 's>>) -> Self {
        handlers.into_iter().fold(self, |settings, entry| match entry {
            HandlerEntry::Element(selector, handlers) => settings.append_element_content_handler((selector, handlers)),
            HandlerEntry::Document(handlers) => settings.append_document_content_handler(handlers),
            HandlerEntry::BailOut(handler) => settings.append_bail_out_handler(handler),
        })
    }
}

#[cfg(test)]
#[path = "settings.test.rs"]
mod tests;
