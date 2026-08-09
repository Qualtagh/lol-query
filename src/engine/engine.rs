use std::cell::RefCell;
use std::rc::Rc;

use lol_html::html_content::{Comment, Element, EndTag, TextChunk};
use lol_html::{HandlerResult, LocalHandlerTypes, comments, element, text};

use crate::ElementView;
use crate::HandlerEntry;
use crate::matcher::{Predicate, Step};
use crate::util::{GenRegExp, Pattern};

use super::{ChainId, InstanceId};

type El<'r, 't> = Element<'r, 't, LocalHandlerTypes>;

pub(crate) trait Callback: for<'r, 't> FnMut(&mut El<'r, 't>) + 'static {}
impl<F: for<'r, 't> FnMut(&mut El<'r, 't>) + 'static> Callback for F {}

pub(crate) trait TextCallback: FnMut(&mut TextChunk<'_>) + 'static {}
impl<F: FnMut(&mut TextChunk<'_>) + 'static> TextCallback for F {}

pub(crate) trait AggregatedTextCallback: FnMut(&str) + 'static {}
impl<F: FnMut(&str) + 'static> AggregatedTextCallback for F {}

pub(crate) trait CommentCallback: FnMut(&mut Comment<'_>) + 'static {}
impl<F: FnMut(&mut Comment<'_>) + 'static> CommentCallback for F {}

/// A [`Step`] compiled into indices into [`State`].
enum Test {
    Leaf(usize),
    Chain(usize),
    Not(usize),
    Every(Vec<usize>),
    Any(Vec<usize>),
}

impl Test {
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

/// A whole chain forest flattened into dependency-ordered ancestry chains.
#[derive(Default)]
struct Program {
    /// Every chain is preceded by the chains it refers to, so a root chain comes last among its deps.
    chains: Vec<CompiledChain>,
    /// The selector and the predicate behind each [`Test::Leaf`].
    leaves: Vec<(String, Option<Box<dyn Predicate>>)>,
}

impl Program {
    fn add(&mut self, steps: Vec<Step>) -> ChainId {
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
            Step::Not(steps) => Test::Not(self.add(steps)),
            Step::Every(chains) => Test::Every(chains.into_iter().map(|steps| self.add(steps)).collect()),
            Step::Any(chains) => Test::Any(chains.into_iter().map(|steps| self.add(steps)).collect()),
            Step::GapWithout(_) | Step::GapWithEvery(_) | Step::GapWithAny(_) | Step::Direct => unreachable!(),
        }
    }

    fn compile_gap(&mut self, step: Step, tests: &mut Vec<Test>) -> Pattern {
        match step {
            Step::Direct => Pattern::epsilon(),
            Step::GapWithout(steps) => {
                let bit = self.add_nested_test(steps, tests);
                Pattern::not_bit(bit).repeat()
            },
            Step::GapWithEvery(chains) => {
                let requirements = chains
                    .into_iter()
                    .map(|steps| {
                        let bit = self.add_nested_test(steps, tests);
                        Pattern::sequence(vec![Pattern::universal(), Pattern::bit(bit), Pattern::universal()])
                    })
                    .collect();
                Pattern::intersection(requirements)
            },
            Step::GapWithAny(chains) => {
                let alternatives = chains
                    .into_iter()
                    .map(|steps| {
                        let bit = self.add_nested_test(steps, tests);
                        Pattern::bit(bit)
                    })
                    .collect();
                Pattern::sequence(vec![Pattern::universal(), Pattern::choice(alternatives), Pattern::universal()])
            },
            Step::Filter(_, _) | Step::Not(_) | Step::Every(_) | Step::Any(_) => unreachable!(),
        }
    }

    fn add_nested_test(&mut self, steps: Vec<Step>, tests: &mut Vec<Test>) -> usize {
        tests.push(Test::Chain(self.add(steps)));
        tests.len() - 1
    }
}

/// `(instance, node_id, depth, element)`.
///
/// `node_id` is the canonical DOM node for this enter (same value for every chain that
/// matches the element). Wrap with plan `NodeId::new` — not an [`InstanceId`].
pub(crate) trait EnterCallback: for<'r, 't> FnMut(InstanceId, u64, u32, &mut El<'r, 't>) + 'static {}
impl<F: for<'r, 't> FnMut(InstanceId, u64, u32, &mut El<'r, 't>) + 'static> EnterCallback for F {}

pub(crate) trait ExitCallback: FnMut(InstanceId) + 'static {}
impl<F: FnMut(InstanceId) + 'static> ExitCallback for F {}

type SharedTextCallback = Rc<RefCell<Box<dyn AggregatedTextCallback>>>;
type SharedExitCallback = Rc<RefCell<Box<dyn ExitCallback>>>;

#[derive(Default)]
struct ChainBindings {
    on_match: Option<Box<dyn Callback>>,
    on_enter: Option<Box<dyn EnterCallback>>,
    on_exit: Option<Box<dyn ExitCallback>>,
    on_text_chunk: Option<Box<dyn TextCallback>>,
    on_text: Option<Box<dyn AggregatedTextCallback>>,
    on_comment: Option<Box<dyn CommentCallback>>,
}

impl ChainBindings {
    fn has_lifecycle(&self) -> bool {
        self.on_match.is_some()
            || self.on_enter.is_some()
            || self.on_exit.is_some()
            || self.on_text_chunk.is_some()
            || self.on_text.is_some()
            || self.on_comment.is_some()
    }

    fn track_text(&self) -> bool {
        self.on_text_chunk.is_some() || self.on_text.is_some()
    }
}

/// Per-depth bookkeeping for sibling axes and subscriptions.
#[derive(Default)]
struct DepthSlot {
    /// Closed-child summaries for `prev` / `+` / `~`.
    _sibling_summary: (),
    /// Pending `next` / `nextAll` subscriptions armed at this depth.
    _subscriptions: (),
}

#[derive(Default)]
struct ChainState {
    /// Still-open matches of this chain that own text callbacks.
    text_open: u32,
    /// For each open aggregated-text match (outermost last): start index into [`State::text_chunks`].
    text_starts: Vec<usize>,
    /// Still-open matches of this chain that own comment callbacks.
    comment_open: u32,
    /// Open instance ids of this chain (outermost last).
    open_instances: Vec<InstanceId>,
}

/// Matching progress shared by all handlers built by one [`Engine`].
struct State {
    /// Per leaf: whether its selector and predicate matched the element currently being handled.
    hits: Vec<bool>,
    /// Per chain: whether it matches the element currently being handled.
    matched: Vec<bool>,
    /// Per chain: the DFA state after consuming the currently open ancestry.
    regexp_states: Vec<usize>,
    /// Number of currently open elements (shared nesting depth).
    depth: u32,
    /// Next instance id to allocate (per-chain activation).
    next_instance_id: InstanceId,
    /// Next plan NodeId raw value to allocate (once per element enter, shared across chains).
    next_node_id: u64,
    /// Per-depth slots for sibling summaries and subscriptions.
    depth_slots: Vec<DepthSlot>,
    /// Text chunks seen under currently open aggregated-text matches; each chunk is stored once.
    text_chunks: Vec<String>,
    /// Per-chain open-match bookkeeping.
    chains: Vec<ChainState>,
}

impl State {
    fn ensure_depth_slot(&mut self, depth: u32) {
        let need = depth as usize + 1;
        if self.depth_slots.len() >= need {
            return;
        }
        self.depth_slots.resize_with(need, DepthSlot::default);
    }
}

/// Compiles ancestry chains and assembles lol-html handlers that drive them.
#[derive(Default)]
pub(crate) struct Engine {
    program: Program,
    bindings: Vec<ChainBindings>,
}

impl Engine {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Compiles `steps` into a chain and returns its id.
    pub(crate) fn add_chain(&mut self, steps: Vec<Step>) -> ChainId {
        let id = self.program.add(steps);
        if self.bindings.len() <= id {
            self.bindings.resize_with(id + 1, ChainBindings::default);
        }
        id
    }

    /// Runs `callback` once for each element matched by `chain`.
    pub(crate) fn on_match(&mut self, chain: ChainId, callback: impl Callback) {
        self.binding(chain).on_match = Some(Box::new(callback));
    }

    /// Runs `callback` when `chain` matches, with instance id and shared node id.
    #[allow(dead_code)]
    pub(crate) fn on_enter(&mut self, chain: ChainId, callback: impl EnterCallback) {
        self.binding(chain).on_enter = Some(Box::new(callback));
    }

    /// Runs `callback` when a matched instance of `chain` ends.
    #[allow(dead_code)]
    pub(crate) fn on_exit(&mut self, chain: ChainId, callback: impl ExitCallback) {
        self.binding(chain).on_exit = Some(Box::new(callback));
    }

    /// Runs `callback` for each text chunk inside elements matched by `chain`.
    pub(crate) fn on_text_chunk(&mut self, chain: ChainId, callback: impl TextCallback) {
        self.binding(chain).on_text_chunk = Some(Box::new(callback));
    }

    /// Runs `callback` once per matched element with the combined descendant text.
    pub(crate) fn on_text(&mut self, chain: ChainId, callback: impl AggregatedTextCallback) {
        self.binding(chain).on_text = Some(Box::new(callback));
    }

    /// Runs `callback` for each HTML comment inside elements matched by `chain`.
    pub(crate) fn on_comment(&mut self, chain: ChainId, callback: impl CommentCallback) {
        self.binding(chain).on_comment = Some(Box::new(callback));
    }

    fn binding(&mut self, chain: ChainId) -> &mut ChainBindings {
        assert!(chain < self.program.chains.len(), "unknown chain id");
        if self.bindings.len() <= chain {
            self.bindings.resize_with(chain + 1, ChainBindings::default);
        }
        &mut self.bindings[chain]
    }

    /// Consumes the engine and returns handler entries for [`crate::SettingsExt::add_handlers`].
    pub(crate) fn into_handlers(self) -> Vec<HandlerEntry<'static, 'static>> {
        let Engine { program, mut bindings } = self;
        let Program { mut chains, leaves } = program;
        if bindings.len() < chains.len() {
            bindings.resize_with(chains.len(), ChainBindings::default);
        }
        let lifecycle: Vec<bool> = bindings.iter().map(ChainBindings::has_lifecycle).collect();
        let track_text_flags: Vec<bool> = bindings.iter().map(ChainBindings::track_text).collect();
        let aggregated_flags: Vec<bool> = bindings.iter().map(|b| b.on_text.is_some()).collect();
        let comment_flags: Vec<bool> = bindings.iter().map(|b| b.on_comment.is_some()).collect();
        let track_text_any = track_text_flags.iter().any(|&flag| flag);
        let has_aggregated_any = aggregated_flags.iter().any(|&flag| flag);
        let has_comment_any = comment_flags.iter().any(|&flag| flag);
        let on_text: Vec<Option<SharedTextCallback>> = bindings.iter_mut().map(|b| b.on_text.take().map(|callback| Rc::new(RefCell::new(callback)))).collect();
        let on_exit: Vec<Option<SharedExitCallback>> = bindings.iter_mut().map(|b| b.on_exit.take().map(|callback| Rc::new(RefCell::new(callback)))).collect();
        let mut on_match: Vec<Option<Box<dyn Callback>>> = bindings.iter_mut().map(|b| b.on_match.take()).collect();
        let mut on_enter: Vec<Option<Box<dyn EnterCallback>>> = bindings.iter_mut().map(|b| b.on_enter.take()).collect();
        let mut on_text_chunk: Vec<Option<Box<dyn TextCallback>>> = bindings.iter_mut().map(|b| b.on_text_chunk.take()).collect();
        let mut on_comment: Vec<Option<Box<dyn CommentCallback>>> = bindings.iter_mut().map(|b| b.on_comment.take()).collect();
        let shared = Rc::new(RefCell::new(State {
            hits: vec![false; leaves.len()],
            matched: vec![false; chains.len()],
            regexp_states: chains.iter().map(|chain| chain.regexp.start_state()).collect(),
            depth: 0,
            next_instance_id: 0,
            next_node_id: 0,
            depth_slots: vec![DepthSlot::default()],
            text_chunks: vec![],
            chains: (0..chains.len()).map(|_| ChainState::default()).collect(),
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
                    if let Some(predicate) = &predicate && !predicate(&ElementView::new(el)) {
                        return Ok(());
                    }
                    shared.borrow_mut().hits[leaf] = true;
                    Ok(())
                })
                .into()
            })
            .collect();
        let shared_for_combine = shared.clone();
        let aggregated_flags_for_text = aggregated_flags.clone();
        let combine = element!("*", move |el: &mut El<'_, '_>| -> HandlerResult {
            let mut pending: Vec<(usize, InstanceId, u64, u32, bool)> = Vec::new();
            {
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
                let parent_depth = state.depth;
                let child_depth = parent_depth + 1;
                // One NodeId raw value per element enter — shared by every chain that matches.
                let node_id = state.next_node_id;
                state.next_node_id += 1;
                if let Some(end_tag_handlers) = el.end_tag_handlers() {
                    state.regexp_states = child_states;
                    state.depth = child_depth;
                    state.ensure_depth_slot(child_depth);
                    let shared = shared_for_combine.clone();
                    end_tag_handlers.push(Box::new(move |_: &mut EndTag<'_>| {
                        let state = &mut *shared.borrow_mut();
                        state.regexp_states = parent_states;
                        state.depth = parent_depth;
                        Ok(())
                    }));
                }
                for chain_id in 0..lifecycle.len() {
                    if !state.matched[chain_id] || !lifecycle[chain_id] {
                        continue;
                    }
                    let instance = state.next_instance_id;
                    state.next_instance_id += 1;
                    let Some(end_tag_handlers) = el.end_tag_handlers() else {
                        pending.push((chain_id, instance, node_id, child_depth, true));
                        continue;
                    };
                    state.chains[chain_id].open_instances.push(instance);
                    let track_text = track_text_flags[chain_id];
                    let has_aggregated_text = aggregated_flags[chain_id];
                    let has_comment = comment_flags[chain_id];
                    if track_text {
                        state.chains[chain_id].text_open += 1;
                        if has_aggregated_text {
                            state.chains[chain_id].text_starts.push(state.text_chunks.len());
                        }
                    }
                    if has_comment {
                        state.chains[chain_id].comment_open += 1;
                    }
                    let shared = shared_for_combine.clone();
                    let on_text = on_text[chain_id].clone();
                    let on_exit = on_exit[chain_id].clone();
                    end_tag_handlers.push(Box::new(move |_: &mut EndTag<'_>| {
                        let (ended, text) = {
                            let state = &mut *shared.borrow_mut();
                            let ended = state.chains[chain_id].open_instances.pop().unwrap();
                            // Aggregated text is ready before exit so End-scheduled plan nodes can read it.
                            let text = if track_text && has_aggregated_text {
                                let start = state.chains[chain_id].text_starts.pop().unwrap();
                                let len = state.text_chunks[start..].iter().map(String::len).sum();
                                let mut text = String::with_capacity(len);
                                for chunk in &state.text_chunks[start..] {
                                    text.push_str(chunk);
                                }
                                let still_needed = state.chains.iter().any(|chain| !chain.text_starts.is_empty());
                                if !still_needed {
                                    state.text_chunks.clear();
                                }
                                Some(text)
                            } else {
                                None
                            };
                            if track_text {
                                state.chains[chain_id].text_open -= 1;
                            }
                            if has_comment {
                                state.chains[chain_id].comment_open -= 1;
                            }
                            (ended, text)
                        };
                        if let Some(text) = &text {
                            on_text.as_ref().unwrap().borrow_mut()(text);
                        }
                        if let Some(callback) = &on_exit {
                            callback.borrow_mut()(ended);
                        }
                        Ok(())
                    }));
                    pending.push((chain_id, instance, node_id, child_depth, false));
                }
            }
            for (chain_id, instance, node_id, depth, exit_now) in pending {
                if let Some(callback) = on_match[chain_id].as_mut() {
                    callback(el);
                }
                if let Some(callback) = on_enter[chain_id].as_mut() {
                    callback(instance, node_id, depth, el);
                }
                if !exit_now {
                    continue;
                }
                if let Some(callback) = &on_exit[chain_id] {
                    callback.borrow_mut()(instance);
                }
            }
            Ok(())
        });
        handlers.push(combine.into());
        if track_text_any {
            let shared = shared.clone();
            handlers.push(
                text!("*", move |chunk: &mut TextChunk<'_>| -> HandlerResult {
                    let open: Vec<bool> = shared.borrow().chains.iter().map(|chain| chain.text_open > 0).collect();
                    if !open.iter().any(|&is_open| is_open) {
                        return Ok(());
                    }
                    for (chain_id, callback) in on_text_chunk.iter_mut().enumerate() {
                        let Some(callback) = callback else {
                            continue;
                        };
                        if !open[chain_id] {
                            continue;
                        }
                        callback(chunk);
                    }
                    if has_aggregated_any {
                        let store = open.iter().enumerate().any(|(chain_id, &is_open)| is_open && aggregated_flags_for_text[chain_id]);
                        if store {
                            shared.borrow_mut().text_chunks.push(chunk.as_str().to_owned());
                        }
                    }
                    Ok(())
                })
                .into(),
            );
        }
        if has_comment_any {
            let shared = shared.clone();
            handlers.push(
                comments!("*", move |comment: &mut Comment<'_>| -> HandlerResult {
                    let open: Vec<bool> = shared.borrow().chains.iter().map(|chain| chain.comment_open > 0).collect();
                    if !open.iter().any(|&is_open| is_open) {
                        return Ok(());
                    }
                    for (chain_id, callback) in on_comment.iter_mut().enumerate() {
                        let Some(callback) = callback else {
                            continue;
                        };
                        if !open[chain_id] {
                            continue;
                        }
                        callback(comment);
                    }
                    Ok(())
                })
                .into(),
            );
        }
        handlers
    }
}

#[cfg(test)]
#[path = "engine.test.rs"]
mod tests;
