#[derive(Debug, Clone)]
pub(super) enum RequestedBreakpoint {
    Source { path: String, line: u64 },
    Function(String),
    Instruction(String),
}

pub(super) fn parse(value: &str) -> RequestedBreakpoint {
    if let Some(reference) = value.strip_prefix('*') {
        return RequestedBreakpoint::Instruction(reference.to_owned());
    }
    if value.starts_with("0x") {
        return RequestedBreakpoint::Instruction(value.to_owned());
    }
    if let Some((path, line)) = value.rsplit_once(':')
        && !path.is_empty()
        && let Ok(line) = line.parse()
    {
        return RequestedBreakpoint::Source {
            path: path.to_owned(),
            line,
        };
    }
    RequestedBreakpoint::Function(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_forms() {
        assert!(matches!(
            parse("src/main.rs:42"),
            RequestedBreakpoint::Source { path, line } if path == "src/main.rs" && line == 42
        ));
        assert!(matches!(
            parse("main"),
            RequestedBreakpoint::Function(name) if name == "main"
        ));
        assert!(matches!(
            parse("*0x1234"),
            RequestedBreakpoint::Instruction(reference) if reference == "0x1234"
        ));
    }
}
