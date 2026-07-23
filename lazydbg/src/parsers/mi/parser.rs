use crate::parsers::mi::record::{AsyncKind, Record, ResultClass, StreamKind};
use crate::parsers::mi::value::Value;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{escaped_transform, tag, take_while1},
    character::complete::{char, digit1, none_of},
    combinator::{map, map_res, opt, recognize, value as nom_value},
    multi::{many0, separated_list0},
    sequence::{delimited, preceded, separated_pair},
};
use std::collections::HashMap;

fn token(i: &str) -> IResult<&str, u64> {
    map_res(digit1, str::parse).parse(i)
}
fn opt_token(i: &str) -> IResult<&str, Option<u64>> {
    opt(token).parse(i)
}

fn identifier(i: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-').parse(i)
}

fn escaped_char(i: &str) -> IResult<&str, &str> {
    alt((
        nom_value("\\", char('\\')),
        nom_value("\"", char('"')),
        nom_value("\n", char('n')),
        nom_value("\t", char('t')),
        nom_value("\r", char('r')),
        recognize(nom::character::complete::anychar), // unknown escape: pass through
    ))
    .parse(i)
}

fn c_string(i: &str) -> IResult<&str, String> {
    delimited(
        char('"'),
        map(
            opt(escaped_transform(none_of("\\\""), '\\', escaped_char)),
            |s: Option<String>| s.unwrap_or_default(),
        ),
        char('"'),
    )
    .parse(i)
}

fn value(i: &str) -> IResult<&str, Value> {
    alt((map(c_string, Value::Str), tuple_val, list_val)).parse(i)
}

fn result_pair(i: &str) -> IResult<&str, (String, Value)> {
    map(separated_pair(identifier, char('='), value), |(k, v)| {
        (k.to_string(), v)
    })
    .parse(i)
}

fn tuple_val(i: &str) -> IResult<&str, Value> {
    map(
        delimited(
            char('{'),
            separated_list0(char(','), result_pair),
            char('}'),
        ),
        |pairs| Value::Tuple(pairs.into_iter().collect()),
    )
    .parse(i)
}

// List elements can be bare values OR "name=value" pairs (e.g. thread-ids lists).
fn list_element(i: &str) -> IResult<&str, Value> {
    alt((
        map(result_pair, |(k, v)| Value::Tuple(HashMap::from([(k, v)]))),
        value,
    ))
    .parse(i)
}

fn list_val(i: &str) -> IResult<&str, Value> {
    map(
        delimited(
            char('['),
            separated_list0(char(','), list_element),
            char(']'),
        ),
        Value::List,
    )
    .parse(i)
}

fn results(i: &str) -> IResult<&str, HashMap<String, Value>> {
    map(many0(preceded(char(','), result_pair)), |v| {
        v.into_iter().collect()
    })
    .parse(i)
}

fn result_class(i: &str) -> IResult<&str, ResultClass> {
    alt((
        nom_value(ResultClass::Done, tag("done")),
        nom_value(ResultClass::Running, tag("running")),
        nom_value(ResultClass::Connected, tag("connected")),
        nom_value(ResultClass::Error, tag("error")),
        nom_value(ResultClass::Exit, tag("exit")),
    ))
    .parse(i)
}

fn result_record(i: &str) -> IResult<&str, Record> {
    map(
        // nom 8: tuples of parsers implement `Parser` directly (sequence::tuple is deprecated)
        (opt_token, preceded(char('^'), result_class), results),
        |(token, class, results)| Record::Result {
            token,
            class,
            results,
        },
    )
    .parse(i)
}

fn async_kind_and_class(i: &str) -> IResult<&str, (AsyncKind, &str)> {
    alt((
        map(preceded(char('*'), identifier), |c| (AsyncKind::Exec, c)),
        map(preceded(char('+'), identifier), |c| (AsyncKind::Status, c)),
        map(preceded(char('='), identifier), |c| (AsyncKind::Notify, c)),
    ))
    .parse(i)
}

fn async_record(i: &str) -> IResult<&str, Record> {
    map(
        (opt_token, async_kind_and_class, results),
        |(token, (kind, class), results)| Record::Async {
            token,
            kind,
            class: class.to_string(),
            results,
        },
    )
    .parse(i)
}

fn stream_record(i: &str) -> IResult<&str, Record> {
    alt((
        map(preceded(char('~'), c_string), |t| Record::Stream {
            kind: StreamKind::Console,
            text: t,
        }),
        map(preceded(char('@'), c_string), |t| Record::Stream {
            kind: StreamKind::Target,
            text: t,
        }),
        map(preceded(char('&'), c_string), |t| Record::Stream {
            kind: StreamKind::Log,
            text: t,
        }),
    ))
    .parse(i)
}

/// Top-level: parse one MI record from one line of text.
pub fn record(i: &str) -> IResult<&str, Record> {
    alt((
        nom_value(Record::Prompt, tag("(gdb)")),
        result_record,
        async_record,
        stream_record,
    ))
    .parse(i)
}
