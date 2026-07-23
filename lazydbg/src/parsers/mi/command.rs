use crate::parsers::mi::Record;
use crate::parsers::mi::ser::{ArgValue, CommandSerializer, Error};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Implement per MI command. `OP` is the only hardcoded string per command.
pub trait MiCommand: Serialize {
    const OP: &'static str;
    /// Set true for commands whose grammar requires "--" between options and
    /// positional args (e.g. `-data-disassemble ... -- mode`).
    const DASH_DASH_BEFORE_POSITIONAL: bool = false;
    type Reply: DeserializeOwned + Serialize;

    /// Parse the results of a record into this command's Reply type.
    fn parse_reply(record: &Record) -> Option<Result<Self::Reply, crate::parsers::mi::de::Error>> {
        record.parse_results::<Self::Reply>()
    }
}

/// Field naming convention:
/// - `#[serde(rename = "c")] condition: Option<String>` -> `-c <value>` (GDB's real short-opt style)
/// - unrenamed `foo_bar: T` -> `-foo-bar <value>` (underscores become dashes, fallback for rare long opts)
/// - `bool` -> bare `-flag` if true, omitted if false
/// - `Option<T>` -> omitted if None
/// - field named exactly `positional: String | Option<String> | Vec<String>` -> appended unlabeled, in order, at the end
pub fn build_line<C: MiCommand>(cmd: &C, token: u64) -> Result<String, Error> {
    let fields = cmd.serialize(CommandSerializer)?;
    let mut out = format!("{token}-{}", C::OP);
    let mut positional: Vec<String> = Vec::new();

    for (name, val) in fields {
        if name == "positional" {
            collect_positional(val, &mut positional);
            continue;
        }
        let flag = format!("-{}", name.replace('_', "-"));
        match val {
            ArgValue::Null | ArgValue::Bool(false) => {}
            ArgValue::Bool(true) => {
                out.push(' ');
                out.push_str(&flag);
            }
            ArgValue::Str(s) => {
                out.push(' ');
                out.push_str(&flag);
                out.push(' ');
                out.push_str(&quote(&s));
            }
            ArgValue::Seq(items) => {
                for item in items {
                    if let ArgValue::Str(s) = item {
                        out.push(' ');
                        out.push_str(&flag);
                        out.push(' ');
                        out.push_str(&quote(&s));
                    }
                }
            }
        }
    }
    if C::DASH_DASH_BEFORE_POSITIONAL && !positional.is_empty() {
        out.push_str(" --");
    }
    for p in positional {
        out.push(' ');
        out.push_str(&p);
    }
    out.push('\n');
    Ok(out)
}

fn collect_positional(val: ArgValue, out: &mut Vec<String>) {
    match val {
        ArgValue::Str(s) => out.push(quote(&s)),
        ArgValue::Seq(items) => {
            for i in items {
                collect_positional(i, out);
            }
        }
        _ => {}
    }
}

fn quote(s: &str) -> String {
    if s.is_empty() || s.contains(' ') {
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        s.to_string()
    }
}
