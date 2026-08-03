//! Offline document store over [`scraper`].
//!
//! Navigation follows the planner element algebra: axes skip text and comments.
//! The synthetic plan root is scraper's document or fragment node.

use std::collections::HashMap;

use scraper::node::Node;
use scraper::{ElementRef, Html, Selector};

use crate::plan::representation::id::NodeId;

type EgoId = ego_tree::NodeId;

/// Parsed HTML document (or fragment) with plan [`NodeId`]s.
#[derive(Debug, Clone)]
pub(crate) struct Dom {
    html: Html,
    /// Plan id → scraper ego-tree id. Dense from 0.
    by_plan: Vec<EgoId>,
    /// Scraper ego-tree id → plan id raw.
    by_ego: HashMap<EgoId, u64>,
    root: NodeId,
}

impl Dom {
    /// Parse a full HTML document.
    pub(crate) fn parse_document(html: &str) -> Self {
        Self::from_html(Html::parse_document(html))
    }

    /// Parse an HTML fragment (scraper wraps it under a synthetic `<html>`).
    pub(crate) fn parse_fragment(html: &str) -> Self {
        Self::from_html(Html::parse_fragment(html))
    }

    fn from_html(html: Html) -> Self {
        let mut by_plan = Vec::new();
        let mut by_ego = HashMap::new();

        for node in html.tree.nodes() {
            let value = node.value();
            if !(value.is_document() || value.is_fragment() || value.is_element()) {
                continue;
            }
            let ego = node.id();
            by_ego.insert(ego, by_plan.len() as u64);
            by_plan.push(ego);
        }

        let root_ego = html.tree.root().id();
        let root = NodeId::new(*by_ego.get(&root_ego).expect("document/fragment root indexed"));

        Self { html, by_plan, by_ego, root }
    }

    /// Synthetic plan root (not an HTML element).
    pub(crate) fn root(&self) -> NodeId {
        self.root
    }

    /// Parent along: nearest element ancestor, or root for top-level elements.
    pub(crate) fn parent(&self, id: NodeId) -> Option<NodeId> {
        if id == self.root {
            return None;
        }
        let mut cursor = self.node_ref(id)?.parent()?;
        loop {
            match cursor.value() {
                Node::Document | Node::Fragment => return Some(self.root),
                Node::Element(_) => return Some(self.plan_id(cursor.id())),
                _ => cursor = cursor.parent()?,
            }
        }
    }

    /// First element child along.
    pub(crate) fn first_child(&self, id: NodeId) -> Option<NodeId> {
        self.node_ref(id)?.children().find(|child| child.value().is_element()).map(|child| self.plan_id(child.id()))
    }

    /// Next element sibling along.
    pub(crate) fn next_sibling(&self, id: NodeId) -> Option<NodeId> {
        if id == self.root {
            return None;
        }
        self.node_ref(id)?.next_siblings().find(|sib| sib.value().is_element()).map(|sib| self.plan_id(sib.id()))
    }

    /// Previous element sibling along.
    pub(crate) fn prev_sibling(&self, id: NodeId) -> Option<NodeId> {
        if id == self.root {
            return None;
        }
        self.node_ref(id)?.prev_siblings().find(|sib| sib.value().is_element()).map(|sib| self.plan_id(sib.id()))
    }

    /// Element tag name in lowercase, or `None` for root.
    pub(crate) fn tag(&self, id: NodeId) -> Option<&str> {
        self.element(id).map(|el| el.value().name())
    }

    /// Attribute value, or `None` if missing or not an element.
    pub(crate) fn attr<'a>(&'a self, id: NodeId, name: &str) -> Option<&'a str> {
        self.element(id)?.attr(name)
    }

    /// Cheerio-style aggregated descendant text. Empty for root.
    pub(crate) fn text(&self, id: NodeId) -> String {
        let Some(el) = self.element(id) else {
            return String::new();
        };
        el.text().collect()
    }

    /// Whether the node matches a CSS selector. Always `false` for root.
    pub(crate) fn matches(&self, id: NodeId, selector: &Selector) -> bool {
        let Some(el) = self.element(id) else {
            return false;
        };
        selector.matches(&el)
    }

    fn plan_id(&self, ego: EgoId) -> NodeId {
        NodeId::new(*self.by_ego.get(&ego).expect("navigated to indexed node"))
    }

    fn ego_id(&self, id: NodeId) -> Option<EgoId> {
        self.by_plan.get(id.raw() as usize).copied()
    }

    fn node_ref(&self, id: NodeId) -> Option<ego_tree::NodeRef<'_, Node>> {
        let ego = self.ego_id(id)?;
        self.html.tree.get(ego)
    }

    fn element(&self, id: NodeId) -> Option<ElementRef<'_>> {
        ElementRef::wrap(self.node_ref(id)?)
    }
}

#[cfg(test)]
#[path = "dom.test.rs"]
mod test;
