use crate::command::MiCommand;
use crate::value::EmptyReply;
use serde::Serialize;

/// `-file-exec-and-symbols [file]`
#[derive(Serialize, Default)]
pub struct FileExecAndSymbols {
    pub positional: Option<String>,
}
impl MiCommand for FileExecAndSymbols {
    const OP: &'static str = "file-exec-and-symbols";
    type Reply = EmptyReply;
}

/// `-file-exec-file [file]`
#[derive(Serialize, Default)]
pub struct FileExecFile {
    pub positional: Option<String>,
}
impl MiCommand for FileExecFile {
    const OP: &'static str = "file-exec-file";
    type Reply = EmptyReply;
}

/// `-file-list-exec-sections`
#[derive(Serialize, Default)]
pub struct FileListExecSections {}
impl MiCommand for FileListExecSections {
    const OP: &'static str = "file-list-exec-sections";
    type Reply = crate::Value;
}

/// `-file-list-exec-source-files`
#[derive(Serialize, Default)]
pub struct FileListExecSourceFiles {}
impl MiCommand for FileListExecSourceFiles {
    const OP: &'static str = "file-list-exec-source-files";
    type Reply = crate::Value;
}

/// `-file-list-shared-libraries`
#[derive(Serialize, Default)]
pub struct FileListSharedLibraries {}
impl MiCommand for FileListSharedLibraries {
    const OP: &'static str = "file-list-shared-libraries";
    type Reply = crate::Value;
}

/// `-file-list-symbol-files`
#[derive(Serialize, Default)]
pub struct FileListSymbolFiles {}
impl MiCommand for FileListSymbolFiles {
    const OP: &'static str = "file-list-symbol-files";
    type Reply = crate::Value;
}

/// `-file-symbol-file [file]`
#[derive(Serialize, Default)]
pub struct FileSymbolFile {
    pub positional: Option<String>,
}
impl MiCommand for FileSymbolFile {
    const OP: &'static str = "file-symbol-file";
    type Reply = EmptyReply;
}
