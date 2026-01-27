pub use lexopt::Parser;

// Reset the parser at the current position, but with a new context.
pub fn reset_parser(mut parser: Parser) -> anyhow::Result<Parser> {
    Ok(lexopt::Parser::from_args(parser.raw_args()?))
}

pub fn next_maybe_num(parser: &'_ mut Parser) -> anyhow::Result<Option<lexopt::Arg<'_>>> {
    if let Some(number) = try_get_number(parser) {
        Ok(Some(lexopt::Arg::Value(number.into())))
    } else {
        Ok(parser.next()?)
    }
}

pub fn try_get_number(parser: &mut Parser) -> Option<String> {
    let mut raw = parser.try_raw_args()?;
    let arg = raw.peek()?.to_str()?;
    if arg.starts_with('-') && arg[1..].starts_with(|c: char| c.is_ascii_digit()) {
        raw.next()
            .map(|os_string| os_string.to_string_lossy().into())
    } else {
        None
    }
}

#[allow(dead_code)]
pub fn positional<T: std::str::FromStr>(parser: &mut Parser, name: &str) -> anyhow::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
    Result<T, anyhow::Error>: From<Result<T, <T as std::str::FromStr>::Err>>,
{
    if let Some(os_string) = parser.raw_args()?.next() {
        Ok(os_string.to_string_lossy().parse::<T>()?)
    } else {
        Err(anyhow::Error::msg(format!(
            "missing positional argument '{name}'"
        )))
    }
}

pub fn to_window_id(s: &str) -> Option<String> {
    if s.starts_with('%') || s.starts_with('{') {
        Some(s.into())
    } else {
        None
    }
}
