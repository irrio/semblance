use crate::{
    inst::{WasmTrap, WasmValue, table::WasmInstanceAddr},
    module::{WasmInstruction, WasmLabelIdx},
};

pub struct WasmStack {
    value_stack: WasmValueStack,
    control_stack: Vec<ControlStackEntry>,
    max_control_stack_depth: usize,
}

pub enum ControlStackEntry {
    Frame(WasmFrame),
    Label(WasmLabel),
}

pub struct WasmLabel {
    pub instr: *const WasmInstruction,
}

pub struct WasmFrame {
    pub locals: Box<[WasmValue]>,
    pub winst_id: WasmInstanceAddr,
}

struct WasmValueStack(Vec<WasmValue>);

impl WasmValueStack {
    pub fn new() -> Self {
        WasmValueStack(Vec::new())
    }

    pub fn push<I: Into<WasmValue>>(&mut self, val: I) {
        self.0.push(val.into())
    }

    pub fn pop(&mut self) -> WasmValue {
        self.0.pop().expect("value stack underflow")
    }
}

impl WasmStack {
    pub fn new(max_control_stack_depth: usize) -> Self {
        WasmStack {
            value_stack: WasmValueStack::new(),
            control_stack: Vec::new(),
            max_control_stack_depth,
        }
    }

    pub fn push_value<V: Into<WasmValue>>(&mut self, val: V) {
        self.value_stack.push(val);
    }

    pub fn pop_value(&mut self) -> WasmValue {
        self.value_stack.pop()
    }

    pub fn pop_values(&mut self, n: usize) -> Vec<WasmValue> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(self.pop_value());
        }
        out.reverse();
        out
    }

    pub fn truncate_values_within(&mut self, arity: usize, drop: usize) {
        let mut popped = Vec::with_capacity(arity);
        for _ in 0..arity {
            popped.push(self.value_stack.0.pop().expect("value stack underflow"));
        }
        for _ in 0..drop {
            self.value_stack.0.pop();
        }
        for v in popped.into_iter().rev() {
            self.value_stack.0.push(v);
        }
    }

    pub fn push_label(&mut self, label: WasmLabel) -> Result<(), WasmTrap> {
        if self.control_stack.len() >= self.max_control_stack_depth {
            return Err(WasmTrap("resource exhaustion"));
        }
        self.control_stack.push(ControlStackEntry::Label(label));
        Ok(())
    }

    pub fn push_frame(&mut self, frame: WasmFrame) -> Result<(), WasmTrap> {
        if self.control_stack.len() >= self.max_control_stack_depth {
            return Err(WasmTrap("resource exhaustion"));
        }
        self.control_stack.push(ControlStackEntry::Frame(frame));
        Ok(())
    }

    pub fn pop_control(&mut self) -> Option<ControlStackEntry> {
        self.control_stack.pop()
    }

    pub fn peek_control(&self) -> Option<&ControlStackEntry> {
        self.control_stack.last()
    }

    pub fn pop_label(&mut self, label_idx: WasmLabelIdx) -> WasmLabel {
        let n = label_idx.0 + 1;
        self.control_stack
            .truncate(self.control_stack.len() - (n - 1) as usize);
        if let Some(ControlStackEntry::Label(label)) = self.control_stack.pop() {
            label
        } else {
            panic!("invalid labelidx");
        }
    }

    pub fn pop_frame(&mut self) -> WasmFrame {
        loop {
            match self.control_stack.pop() {
                Some(ControlStackEntry::Frame(frame)) => return frame,
                Some(ControlStackEntry::Label(_)) => continue,
                None => break,
            }
        }
        panic!("no call frame");
    }

    pub fn current_frame(&self) -> &WasmFrame {
        for entry in self.control_stack.iter().rev() {
            if let ControlStackEntry::Frame(frame) = entry {
                return frame;
            }
        }
        panic!("no call frame");
    }

    pub fn current_frame_mut(&mut self) -> &mut WasmFrame {
        for entry in self.control_stack.iter_mut().rev() {
            if let ControlStackEntry::Frame(frame) = entry {
                return frame;
            }
        }
        panic!("no call frame");
    }
}
