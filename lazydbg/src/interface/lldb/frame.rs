use lazydbg_dap::StackFrame;

use crate::interface::backend::DbgFrame;

#[derive(Clone)]
pub(super) struct LldbFrame {
    level: usize,
    frame: StackFrame,
}

impl LldbFrame {
    pub(super) fn new(level: usize, frame: StackFrame) -> Self {
        Self { level, frame }
    }
}

impl DbgFrame for LldbFrame {
    fn level(&self) -> Option<String> {
        Some(self.level.to_string())
    }

    fn addr(&self) -> Option<String> {
        self.frame.instruction_pointer_reference.clone()
    }

    fn func(&self) -> Option<String> {
        Some(self.frame.name.clone())
    }

    fn file(&self) -> Option<String> {
        self.frame
            .source
            .as_ref()
            .and_then(|source| source.path.clone().or_else(|| source.name.clone()))
    }

    fn line(&self) -> Option<String> {
        (self.frame.line != 0).then(|| self.frame.line.to_string())
    }

    fn clone_box(&self) -> Box<dyn DbgFrame> {
        Box::new(self.clone())
    }
}
