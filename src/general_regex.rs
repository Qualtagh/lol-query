use std::collections::HashMap;

/// A regular-expression atom evaluated against one bitset in the input word.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum Atom {
    Any,
    Bit(usize),
    NotBit(usize),
}

impl Atom {
    fn matches(&self, symbol: &[u64]) -> bool {
        let bit = match self {
            Atom::Any => return true,
            Atom::Bit(bit) | Atom::NotBit(bit) => *bit,
        };
        let is_set = symbol.get(bit / u64::BITS as usize).is_some_and(|word| word & 1 << (bit % u64::BITS as usize) != 0);
        if matches!(self, Atom::NotBit(_)) { !is_set } else { is_set }
    }
}

/// A canonicalized generalized regular expression.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum Expression {
    Never,
    Epsilon,
    Universal,
    Atom(Atom),
    Choice(Vec<Expression>),
    Intersection(Vec<Expression>),
    Sequence(Vec<Expression>),
    Repeat(Box<Expression>),
}

impl Expression {
    fn choice(expressions: Vec<Self>) -> Self {
        let mut flattened = vec![];
        for expression in expressions {
            match expression {
                Expression::Never => {},
                Expression::Universal => return Expression::Universal,
                Expression::Choice(expressions) => flattened.extend(expressions),
                expression => flattened.push(expression),
            }
        }
        flattened.sort_unstable();
        flattened.dedup();
        match flattened.len() {
            0 => Expression::Never,
            1 => flattened.pop().unwrap(),
            _ => Expression::Choice(flattened),
        }
    }

    fn intersection(expressions: Vec<Self>) -> Self {
        let mut flattened = vec![];
        for expression in expressions {
            match expression {
                Expression::Never => return Expression::Never,
                Expression::Universal => {},
                Expression::Intersection(expressions) => flattened.extend(expressions),
                expression => flattened.push(expression),
            }
        }
        if flattened.iter().any(|expression| matches!(expression, Expression::Epsilon)) {
            return if flattened.iter().all(Expression::nullable) { Expression::Epsilon } else { Expression::Never };
        }
        flattened.sort_unstable();
        flattened.dedup();
        match flattened.len() {
            0 => Expression::Universal,
            1 => flattened.pop().unwrap(),
            _ => Expression::Intersection(flattened),
        }
    }

    fn sequence(expressions: Vec<Self>) -> Self {
        let mut flattened = vec![];
        for expression in expressions {
            match expression {
                Expression::Never => return Expression::Never,
                Expression::Epsilon => {},
                Expression::Sequence(expressions) => flattened.extend(expressions),
                expression => flattened.push(expression),
            }
        }
        let mut previous_was_universal = false;
        flattened.retain(|expression| {
            let is_universal = matches!(expression, Expression::Universal);
            let keep = !is_universal || !previous_was_universal;
            previous_was_universal = is_universal;
            keep
        });
        match flattened.len() {
            0 => Expression::Epsilon,
            1 => flattened.pop().unwrap(),
            _ => Expression::Sequence(flattened),
        }
    }

    fn repeat(expression: Self) -> Self {
        match expression {
            Expression::Never | Expression::Epsilon => Expression::Epsilon,
            Expression::Universal | Expression::Atom(Atom::Any) => Expression::Universal,
            Expression::Repeat(_) => expression,
            expression => Expression::Repeat(Box::new(expression)),
        }
    }

    fn nullable(&self) -> bool {
        match self {
            Expression::Never | Expression::Atom(_) => false,
            Expression::Epsilon | Expression::Universal | Expression::Repeat(_) => true,
            Expression::Choice(expressions) => expressions.iter().any(Expression::nullable),
            Expression::Intersection(expressions) | Expression::Sequence(expressions) => expressions.iter().all(Expression::nullable),
        }
    }

    /// Returns the Brzozowski derivative for one input bitset.
    fn derivative(&self, symbol: &[u64]) -> Self {
        match self {
            Expression::Never | Expression::Epsilon => Expression::Never,
            Expression::Universal => Expression::Universal,
            Expression::Atom(atom) => {
                if atom.matches(symbol) {
                    Expression::Epsilon
                } else {
                    Expression::Never
                }
            },
            Expression::Choice(expressions) => Expression::choice(expressions.iter().map(|expression| expression.derivative(symbol)).collect()),
            Expression::Intersection(expressions) => Expression::intersection(expressions.iter().map(|expression| expression.derivative(symbol)).collect()),
            Expression::Sequence(expressions) => {
                let mut choices = vec![];
                for (index, expression) in expressions.iter().enumerate() {
                    let mut sequence = Vec::with_capacity(expressions.len() - index);
                    sequence.push(expression.derivative(symbol));
                    sequence.extend_from_slice(&expressions[index + 1..]);
                    choices.push(Expression::sequence(sequence));
                    if !expression.nullable() {
                        break;
                    }
                }
                Expression::choice(choices)
            },
            Expression::Repeat(expression) => Expression::sequence(vec![expression.derivative(symbol), Expression::Repeat(expression.clone())]),
        }
    }
}

/// Construction API for a [`GenRegExp`].
#[derive(Clone, Debug)]
pub(crate) struct Pattern(Expression);

impl Pattern {
    /// Matches every word, including the empty word.
    pub(crate) fn universal() -> Self {
        Self(Expression::repeat(Expression::Atom(Atom::Any)))
    }

    /// Matches one bitset containing `bit`.
    pub(crate) fn bit(bit: usize) -> Self {
        Self(Expression::Atom(Atom::Bit(bit)))
    }

    /// Matches one bitset not containing `bit`.
    pub(crate) fn not_bit(bit: usize) -> Self {
        Self(Expression::Atom(Atom::NotBit(bit)))
    }

    pub(crate) fn choice(patterns: Vec<Self>) -> Self {
        Self(Expression::choice(patterns.into_iter().map(|pattern| pattern.0).collect()))
    }

    pub(crate) fn intersection(patterns: Vec<Self>) -> Self {
        Self(Expression::intersection(patterns.into_iter().map(|pattern| pattern.0).collect()))
    }

    pub(crate) fn sequence(patterns: Vec<Self>) -> Self {
        Self(Expression::sequence(patterns.into_iter().map(|pattern| pattern.0).collect()))
    }

    pub(crate) fn repeat(self) -> Self {
        Self(Expression::repeat(self.0))
    }

    /// Matches the empty word only.
    pub(crate) fn epsilon() -> Self {
        Self(Expression::Epsilon)
    }

    #[cfg(test)]
    fn any_symbol() -> Self {
        Self(Expression::Atom(Atom::Any))
    }
}

#[derive(Debug)]
struct DfaState {
    expression: Expression,
    accepting: bool,
}

/// A lazily constructed DFA for regular expressions whose input symbols are
/// vectors of 64-bit words.
///
/// Every state is a canonical
/// [Brzozowski derivative](https://en.wikipedia.org/wiki/Brzozowski_derivative)
/// of the original pattern. Transitions are cached, so advancing an already-seen
/// state and bitset does not recalculate the derivative.
#[derive(Debug)]
pub(crate) struct GenRegExp {
    states: Vec<DfaState>,
    state_ids: HashMap<Expression, usize>,
    transitions: HashMap<(usize, Vec<u64>), usize>,
}

impl GenRegExp {
    pub(crate) fn new(pattern: Pattern) -> Self {
        let expression = pattern.0;
        let accepting = expression.nullable();
        let mut state_ids = HashMap::new();
        state_ids.insert(expression.clone(), 0);
        Self { states: vec![DfaState { expression, accepting }], state_ids, transitions: HashMap::new() }
    }

    pub(crate) fn start_state(&self) -> usize {
        0
    }

    /// Advances `state` by one bitset and returns the resulting DFA state.
    pub(crate) fn transition(&mut self, state: usize, symbol: &[u64]) -> usize {
        #[cfg(test)]
        OPERATION_COUNT.with(|count| count.set(count.get() + 1));

        let key = (state, symbol.to_vec());
        if let Some(&next) = self.transitions.get(&key) {
            return next;
        }

        let derivative = self.states[state].expression.derivative(symbol);
        let next = if let Some(&next) = self.state_ids.get(&derivative) {
            next
        } else {
            let next = self.states.len();
            let accepting = derivative.nullable();
            self.states.push(DfaState { expression: derivative.clone(), accepting });
            self.state_ids.insert(derivative, next);
            next
        };
        self.transitions.insert(key, next);
        next
    }

    pub(crate) fn is_match(&self, state: usize) -> bool {
        self.states[state].accepting
    }

    #[cfg(test)]
    fn matches(&mut self, word: &[Vec<u64>]) -> bool {
        let mut state = self.start_state();
        for symbol in word {
            state = self.transition(state, symbol);
        }
        self.is_match(state)
    }

    #[cfg(test)]
    fn state_count(&self) -> usize {
        self.states.len()
    }
}

#[cfg(test)]
std::thread_local! {
    static OPERATION_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_operation_count() {
    OPERATION_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(crate) fn operation_count() -> usize {
    OPERATION_COUNT.with(std::cell::Cell::get)
}

#[cfg(test)]
#[path = "general_regex.test.rs"]
mod tests;
