use lazydbg_mi::commands::FrameInfo;

use crate::interface::backend::DbgFrame;

impl DbgFrame for FrameInfo {
    fn addr(&self) -> Option<String> {
        self.addr.clone()
    }

    fn func(&self) -> Option<String> {
        self.func.clone()
    }

    fn file(&self) -> Option<String> {
        self.file.clone()
    }

    fn line(&self) -> Option<String> {
        self.line.clone()
    }

    fn level(&self) -> Option<String> {
        self.level.clone()
    }

    fn clone_box(&self) -> Box<dyn DbgFrame> {
        Box::new(self.clone())
    }
}
