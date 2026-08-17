//! Streaming executable + [`StreamingExt`]: handlers + Return cell after rewrite.

use crate::HandlerEntry;
use crate::plan::Plan;
use crate::plan::lower::compile;
use crate::plan::representation::registry::Value;
use crate::plan::runtime::ReturnCell;

/// Executable streaming plan: Engine handlers plus a Return cell.
///
/// Feed [`Self::handlers`] into [`crate::SettingsExt::add_handlers`], run the rewriter,
/// then [`Self::take`] the folded result.
pub(crate) struct Streaming {
    handlers: Option<Vec<HandlerEntry<'static, 'static>>>,
    result: ReturnCell,
}

impl Streaming {
    pub(crate) fn new(handlers: Vec<HandlerEntry<'static, 'static>>, result: ReturnCell) -> Self {
        Self { handlers: Some(handlers), result }
    }

    /// Consume handlers once for [`crate::SettingsExt::add_handlers`].
    pub(crate) fn into_handlers(&mut self) -> Vec<HandlerEntry<'static, 'static>> {
        self.handlers.take().expect("Streaming::handlers already taken")
    }

    /// Materialized Return value after the rewrite ends.
    pub(crate) fn take(&self) -> Value {
        self.result.take()
    }
}

/// Streaming backend: lower the plan onto Engine and return a [`Streaming`] handle.
pub(crate) trait StreamingExt {
    fn streaming(self) -> Streaming;
}

impl StreamingExt for Plan {
    fn streaming(self) -> Streaming {
        compile(self)
    }
}

#[cfg(test)]
#[path = "streaming.test.rs"]
mod test;
