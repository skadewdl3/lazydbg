use crate::{command::MiCommand, value::EmptyReply};
use serde::{Deserialize, Serialize};

/// `-thread-info`
#[derive(Serialize, Default)]
pub struct ThreadInfo {}
impl MiCommand for ThreadInfo {
    const OP: &'static str = "thread-info";
    type Reply = crate::Value;
}

/// `-thread-list-all-threads`
#[derive(Serialize, Default)]
pub struct ThreadListAllThreads {}
impl MiCommand for ThreadListAllThreads {
    const OP: &'static str = "thread-list-all-threads";
    type Reply = crate::Value;
}

/// `-thread-list-ids`
#[derive(Serialize, Default)]
pub struct ThreadListIds {}
impl MiCommand for ThreadListIds {
    const OP: &'static str = "thread-list-ids";
    type Reply = ThreadListIdsReply;
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ThreadListIdsReply {
    #[serde(rename = "thread-ids")]
    pub thread_ids: crate::Value, // empty tuple `{}` or `{thread-id="N",...}`
    #[serde(rename = "number-of-threads")]
    pub number_of_threads: String,
}

/// `-thread-select threadnum`
#[derive(Serialize, Default)]
pub struct ThreadSelect {
    pub positional: String,
}
impl ThreadSelect {
    pub fn new(threadnum: u32) -> Self {
        Self {
            positional: threadnum.to_string(),
        }
    }
}
impl MiCommand for ThreadSelect {
    const OP: &'static str = "thread-select";
    type Reply = ThreadSelectReply;
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ThreadSelectReply {
    #[serde(rename = "new-thread-id")]
    pub new_thread_id: String,
    pub frame: super::stack::FrameInfo,
}
