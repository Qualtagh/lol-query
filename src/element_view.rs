//! Read-only view of an HTML element for filter predicates.

use lol_html::HandlerTypes;
use lol_html::html_content::Element;

/// One attribute exposed to filter predicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeView {
    name: String,
    name_preserve_case: String,
    value: String,
}

impl AttributeView {
    /// Attribute name, ASCII-lowercased.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Attribute name with original case preserved.
    pub fn name_preserve_case(&self) -> &str {
        &self.name_preserve_case
    }

    /// Attribute value (may contain HTML/XML entities).
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Read-only surface for filter predicates.
///
/// Does not expose DOM mutation or end-tag handlers.
pub struct ElementView<'a> {
    inner: &'a dyn ElementOps,
}

impl<'a> ElementView<'a> {
    pub(crate) fn new(inner: &'a dyn ElementOps) -> Self {
        Self { inner }
    }

    /// Tag name, ASCII-lowercased.
    pub fn tag_name(&self) -> String {
        self.inner.tag_name()
    }

    /// Tag name with original case preserved.
    pub fn tag_name_preserve_case(&self) -> String {
        self.inner.tag_name_preserve_case()
    }

    /// Whether the start tag ends with `/>` in the source.
    pub fn is_self_closing(&self) -> bool {
        self.inner.is_self_closing()
    }

    /// Whether the element can have inner content (non-void / not self-closed foreign).
    pub fn can_have_content(&self) -> bool {
        self.inner.can_have_content()
    }

    /// Namespace URI of the element.
    pub fn namespace_uri(&self) -> &'static str {
        self.inner.namespace_uri()
    }

    /// Value of the attribute named `name`, if present.
    pub fn get_attribute(&self, name: &str) -> Option<String> {
        self.inner.get_attribute(name)
    }

    /// Whether an attribute named `name` is present.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.inner.has_attribute(name)
    }

    /// All attributes as owned views.
    pub fn attributes(&self) -> Vec<AttributeView> {
        self.inner.attributes()
    }
}

/// Internal backend operations behind [`ElementView`].
pub(crate) trait ElementOps {
    fn tag_name(&self) -> String;
    fn tag_name_preserve_case(&self) -> String;
    fn is_self_closing(&self) -> bool;
    fn can_have_content(&self) -> bool;
    fn namespace_uri(&self) -> &'static str;
    fn get_attribute(&self, name: &str) -> Option<String>;
    fn has_attribute(&self, name: &str) -> bool;
    fn attributes(&self) -> Vec<AttributeView>;
}

impl<H: HandlerTypes> ElementOps for Element<'_, '_, H> {
    fn tag_name(&self) -> String {
        Element::tag_name(self)
    }

    fn tag_name_preserve_case(&self) -> String {
        Element::tag_name_preserve_case(self)
    }

    fn is_self_closing(&self) -> bool {
        Element::is_self_closing(self)
    }

    fn can_have_content(&self) -> bool {
        Element::can_have_content(self)
    }

    fn namespace_uri(&self) -> &'static str {
        Element::namespace_uri(self)
    }

    fn get_attribute(&self, name: &str) -> Option<String> {
        Element::get_attribute(self, name)
    }

    fn has_attribute(&self, name: &str) -> bool {
        Element::has_attribute(self, name)
    }

    fn attributes(&self) -> Vec<AttributeView> {
        Element::attributes(self)
            .iter()
            .map(|attr| AttributeView { name: attr.name(), name_preserve_case: attr.name_preserve_case(), value: attr.value() })
            .collect()
    }
}
