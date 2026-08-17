//! Evaluate a finalized [`Plan`](crate::plan::Plan) offline (HTML -> value).

use super::dom::Dom;
use super::relational;
use crate::plan::Plan;
use crate::plan::representation::registry::Value;

/// Offline backend: parse HTML and evaluate this plan.
pub(crate) trait EvalExt {
    /// Parse `html` and evaluate the plan on the resulting DOM.
    fn eval(&self, html: &str) -> Value;
}

impl EvalExt for Plan {
    fn eval(&self, html: &str) -> Value {
        let dom = Dom::parse_fragment(html);
        relational::eval(&dom, &self.graph, &self.registry)
    }
}
