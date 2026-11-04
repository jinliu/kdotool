pub use lexopt::Parser;

// Reset the parser at the current position, but with a new context.
pub fn reset_parser(mut parser: Parser) -> anyhow::Result<Parser> {
    Ok(lexopt::Parser::from_args(parser.raw_args()?))
}

pub fn next_maybe_num(parser: &mut Parser) -> anyhow::Result<Option<lexopt::Arg<'_>>> {
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

/// A window geometry in X11 `-geometry` syntax: `[WIDTHxHEIGHT][{+-}X{+-}Y]`.
///
/// A `-` offset is measured from the right/bottom edge of the screen to the
/// right/bottom edge of the window, as in X11 (so `-0-0` is the bottom-right
/// corner, which is different from `+0+0`).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Geometry {
    pub width: Option<u32>,
    pub height: Option<u32>,
    /// (offset, measured_from_right_edge)
    pub x: Option<(u32, bool)>,
    /// (offset, measured_from_bottom_edge)
    pub y: Option<(u32, bool)>,
}

pub fn parse_geometry(s: &str) -> anyhow::Result<Geometry> {
    let err = || {
        anyhow::anyhow!(
            "invalid geometry '{s}' (expected e.g. 800x600, +10+20, -0-0, 800x600-50+100)"
        )
    };
    let mut g = Geometry::default();
    let mut rest = s.trim();
    if rest.is_empty() {
        return Err(err());
    }

    // Optional size part: WIDTHxHEIGHT
    if !rest.starts_with(['+', '-']) {
        let end = rest.find(['+', '-']).unwrap_or(rest.len());
        let (size, tail) = rest.split_at(end);
        let (w, h) = size.split_once(['x', 'X']).ok_or_else(err)?;
        g.width = Some(w.parse().map_err(|_| err())?);
        g.height = Some(h.parse().map_err(|_| err())?);
        if g.width == Some(0) || g.height == Some(0) {
            return Err(err());
        }
        rest = tail;
    }

    // Optional position part: {+-}X{+-}Y
    if !rest.is_empty() {
        let (x_neg, after_sign) = (rest.starts_with('-'), &rest[1..]);
        let y_at = after_sign.find(['+', '-']).ok_or_else(err)?;
        let (x_str, y_part) = after_sign.split_at(y_at);
        let (y_neg, y_str) = (y_part.starts_with('-'), &y_part[1..]);
        g.x = Some((x_str.parse().map_err(|_| err())?, x_neg));
        g.y = Some((y_str.parse().map_err(|_| err())?, y_neg));
    }
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(
        w: Option<u32>,
        h: Option<u32>,
        x: Option<(u32, bool)>,
        y: Option<(u32, bool)>,
    ) -> Geometry {
        Geometry {
            width: w,
            height: h,
            x,
            y,
        }
    }

    #[test]
    fn parses_size_only() {
        assert_eq!(
            parse_geometry("800x600").unwrap(),
            g(Some(800), Some(600), None, None)
        );
        assert_eq!(
            parse_geometry("1024X768").unwrap(),
            g(Some(1024), Some(768), None, None)
        );
    }

    #[test]
    fn parses_position_only() {
        assert_eq!(
            parse_geometry("+10+20").unwrap(),
            g(None, None, Some((10, false)), Some((20, false)))
        );
        assert_eq!(
            parse_geometry("-300-100").unwrap(),
            g(None, None, Some((300, true)), Some((100, true)))
        );
        assert_eq!(
            parse_geometry("+300-100").unwrap(),
            g(None, None, Some((300, false)), Some((100, true)))
        );
    }

    #[test]
    fn minus_zero_is_not_plus_zero() {
        assert_eq!(
            parse_geometry("-0-0").unwrap(),
            g(None, None, Some((0, true)), Some((0, true)))
        );
        assert_eq!(
            parse_geometry("+0+0").unwrap(),
            g(None, None, Some((0, false)), Some((0, false)))
        );
    }

    #[test]
    fn parses_size_and_position() {
        assert_eq!(
            parse_geometry("500x300-100-100").unwrap(),
            g(Some(500), Some(300), Some((100, true)), Some((100, true)))
        );
        assert_eq!(
            parse_geometry("1024x768+50-100").unwrap(),
            g(Some(1024), Some(768), Some((50, false)), Some((100, true)))
        );
    }

    #[test]
    fn rejects_garbage() {
        for bad in [
            "",
            "x",
            "800",
            "800x",
            "x600",
            "0x600",
            "800x600+10",
            "+10",
            "+a+b",
            "800x600++1+2",
            "-10-",
        ] {
            assert!(parse_geometry(bad).is_err(), "should reject {bad:?}");
        }
    }
}
