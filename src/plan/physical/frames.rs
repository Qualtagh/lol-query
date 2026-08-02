use crate::engine::InstanceId;

/// One open match on a stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Frame {
    pub instance: InstanceId,
    pub depth: u32,
    /// False when the match is outside an open parent along this scope's move.
    pub active: bool,
}

impl Frame {
    pub fn new(instance: InstanceId, depth: u32, active: bool) -> Self {
        Self { instance, depth, active }
    }
}

/// Mutable stacks of activation frames (axis runtime state).
#[derive(Debug, Default)]
pub(crate) struct Frames {
    open: Vec<Vec<Frame>>,
}

impl Frames {
    pub fn new(stack_count: usize) -> Self {
        Self { open: (0..stack_count).map(|_| Vec::new()).collect() }
    }

    pub fn push(&mut self, stack: usize, frame: Frame) {
        self.open[stack].push(frame);
    }

    pub fn pop(&mut self, stack: usize) -> Option<Frame> {
        self.open[stack].pop()
    }

    pub fn frame_mut(&mut self, stack: usize, instance: InstanceId) -> &mut Frame {
        self.open[stack].iter_mut().rev().find(|frame| frame.instance == instance).expect("missing frame")
    }
}

#[cfg(test)]
#[path = "frames.test.rs"]
mod test;
