//! `Intl`, step I2: `Intl.NumberFormat` and `Number.prototype.toLocaleString`, on ICU4X's decimal,
//! compact and (experimental) currency and percent formatters. Rounding, digit counts, grouping and
//! sign display follow ECMA-402; scientific and engineering notation are assembled here because
//! ICU4X has no formatter for them. The options arrive as the compiler's `key=value;` string.

use super::*;
use icu::decimal::input::Decimal;
use icu::decimal::options::{
    CompactDecimalFormatterOptions, DecimalFormatterOptions, GroupingStrategy,
};
use icu::decimal::{CompactDecimalFormatter, DecimalFormatter};
use icu::experimental::dimension::currency::CurrencyType;
use icu::experimental::dimension::currency::formatter::CurrencyFormatter;
use icu::experimental::dimension::currency::options::{CurrencyFormatterOptions, CurrencyUsage};
use icu::experimental::dimension::percent::formatter::PercentFormatter;
use icu::locale::Locale;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use fixed_decimal::{FloatPrecision, SignDisplay, SignedRoundingMode, UnsignedRoundingMode};

#[derive(Clone, Copy, PartialEq)]
enum Style {
    Decimal,
    Percent,
    Currency,
}

#[derive(Clone, Copy, PartialEq)]
enum Notation {
    Standard,
    Scientific,
    Engineering,
    Compact,
}

struct NumFmt {
    locale: Locale,
    style: Style,
    currency: Option<String>,
    currency_display: String,
    accounting: bool,
    notation: Notation,
    compact_long: bool,
    sign: SignDisplay,
    grouping: Option<GroupingStrategy>,
    min_int: i16,
    fraction: Option<(i16, i16)>,
    significant: Option<(i16, i16)>,
    mode: SignedRoundingMode,
    strip_zeros: bool,
    /// The fraction digits the style implies when none were given.
    default_fraction: (i16, i16),
}

thread_local! {
    static FORMATS: RefCell<Vec<Rc<NumFmt>>> = const { RefCell::new(Vec::new()) };
    static IDS: RefCell<HashMap<(String, String), usize>> = RefCell::new(HashMap::new());
}

/// Fraction digits of a currency (ISO 4217 minor units).
fn currency_digits(code: &str) -> i16 {
    match code {
        "BIF" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW" | "PYG" | "RWF" | "UGX"
        | "UYI" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
        _ => 2,
    }
}

fn parse_mode(v: &str) -> Result<SignedRoundingMode, String> {
    use SignedRoundingMode::*;
    use UnsignedRoundingMode as U;
    Ok(match v {
        "ceil" => Ceil,
        "floor" => Floor,
        "expand" => Unsigned(U::Expand),
        "trunc" => Unsigned(U::Trunc),
        "halfCeil" => HalfCeil,
        "halfFloor" => HalfFloor,
        "halfExpand" => Unsigned(U::HalfExpand),
        "halfTrunc" => Unsigned(U::HalfTrunc),
        "halfEven" => Unsigned(U::HalfEven),
        _ => {
            return Err("roundingMode must be either \"ceil\", \"floor\", \"expand\", \"trunc\", \"halfCeil\", \"halfFloor\", \"halfExpand\", \"halfTrunc\", or \"halfEven\"".into());
        }
    })
}

fn digits(key: &str, v: &str, lo: i16, hi: i16) -> Result<i16, String> {
    match v.parse::<f64>() {
        Ok(n) if n.is_finite() && n >= lo as f64 && n <= hi as f64 => Ok(n.floor() as i16),
        _ => Err(format!("{key} is out of range")),
    }
}

fn build(locales: &str, options: &str) -> Result<NumFmt, String> {
    let locale = intl_locale(locales)?;
    let (mut style, mut notation) = (Style::Decimal, Notation::Standard);
    let mut f = NumFmt {
        locale,
        style,
        currency: None,
        currency_display: "symbol".into(),
        accounting: false,
        notation,
        compact_long: false,
        sign: SignDisplay::Auto,
        grouping: None,
        min_int: 1,
        fraction: None,
        significant: None,
        mode: SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfExpand),
        strip_zeros: false,
        default_fraction: (0, 3),
    };
    let (mut min_fd, mut max_fd, mut min_sd, mut max_sd) = (None, None, None, None);
    let mut grouping_given = false;
    for kv in options.split(';').filter(|s| !s.is_empty()) {
        let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
        match k {
            "style" => {
                style = match v {
                    "decimal" => Style::Decimal,
                    "percent" => Style::Percent,
                    "currency" => Style::Currency,
                    "unit" => return Err("style \"unit\" is not supported yet".into()),
                    _ => {
                        return Err(
                            "style must be either \"decimal\", \"percent\", \"currency\", or \"unit\""
                                .into(),
                        );
                    }
                }
            }
            "currency" => {
                if v.len() != 3 || !v.bytes().all(|b| b.is_ascii_alphabetic()) {
                    return Err("currency is not a well-formed currency code".into());
                }
                f.currency = Some(v.to_ascii_uppercase());
            }
            "currencyDisplay" => match v {
                "symbol" | "narrowSymbol" | "code" | "name" => f.currency_display = v.into(),
                _ => {
                    return Err(
                        "currencyDisplay must be either \"code\", \"symbol\", or \"name\"".into(),
                    );
                }
            },
            "currencySign" => match v {
                "standard" => f.accounting = false,
                "accounting" => f.accounting = true,
                _ => {
                    return Err("currencySign must be either \"standard\" or \"accounting\"".into());
                }
            },
            "notation" => {
                notation = match v {
                    "standard" => Notation::Standard,
                    "scientific" => Notation::Scientific,
                    "engineering" => Notation::Engineering,
                    "compact" => Notation::Compact,
                    _ => {
                        return Err("notation must be either \"standard\", \"scientific\", \"engineering\", or \"compact\"".into());
                    }
                }
            }
            "compactDisplay" => match v {
                "short" => f.compact_long = false,
                "long" => f.compact_long = true,
                _ => return Err("compactDisplay must be either \"short\" or \"long\"".into()),
            },
            "signDisplay" => {
                f.sign = match v {
                    "auto" => SignDisplay::Auto,
                    "never" => SignDisplay::Never,
                    "always" => SignDisplay::Always,
                    "exceptZero" => SignDisplay::ExceptZero,
                    "negative" => SignDisplay::Negative,
                    _ => {
                        return Err("signDisplay must be either \"auto\", \"never\", \"always\", \"exceptZero\", or \"negative\"".into());
                    }
                }
            }
            "useGrouping" => {
                grouping_given = true;
                f.grouping = match v {
                    "false" => Some(GroupingStrategy::Never),
                    "true" | "always" => Some(GroupingStrategy::Always),
                    "auto" => Some(GroupingStrategy::Auto),
                    "min2" => Some(GroupingStrategy::Min2),
                    _ => return Err("useGrouping must be either \"min2\", \"auto\", \"always\", \"true\", or \"false\"".into()),
                }
            }
            "minimumIntegerDigits" => f.min_int = digits(k, v, 1, 21)?,
            "minimumFractionDigits" => min_fd = Some(digits(k, v, 0, 100)?),
            "maximumFractionDigits" => max_fd = Some(digits(k, v, 0, 100)?),
            "minimumSignificantDigits" => min_sd = Some(digits(k, v, 1, 21)?),
            "maximumSignificantDigits" => max_sd = Some(digits(k, v, 1, 21)?),
            "roundingMode" => f.mode = parse_mode(v)?,
            "trailingZeroDisplay" => match v {
                "auto" => f.strip_zeros = false,
                "stripIfInteger" => f.strip_zeros = true,
                _ => {
                    return Err(
                        "trailingZeroDisplay must be either \"auto\" or \"stripIfInteger\"".into(),
                    );
                }
            },
            _ => {}
        }
    }
    if style == Style::Currency && f.currency.is_none() {
        return Err("TypeError: currency must be a string".into());
    }
    f.style = style;
    f.notation = notation;
    f.default_fraction = match style {
        Style::Decimal => (0, 3),
        Style::Percent => (0, 0),
        Style::Currency => {
            let d = currency_digits(f.currency.as_deref().unwrap_or("USD"));
            (d, d)
        }
    };
    if min_sd.is_some() || max_sd.is_some() {
        let lo = min_sd.unwrap_or(1);
        let hi = max_sd.unwrap_or(21);
        if lo > hi {
            return Err("maximumSignificantDigits is out of range".into());
        }
        f.significant = Some((lo, hi));
    } else if min_fd.is_some() || max_fd.is_some() {
        let (dmin, dmax) = f.default_fraction;
        let (lo, hi) = match (min_fd, max_fd) {
            (Some(a), Some(b)) => {
                if a > b {
                    return Err(
                        "Computed minimumFractionDigits is larger than maximumFractionDigits"
                            .into(),
                    );
                }
                (a, b)
            }
            (Some(a), None) => (a, dmax.max(a)),
            (None, Some(b)) => (dmin.min(b), b),
            _ => unreachable!(),
        };
        f.fraction = Some((lo, hi));
    }
    if !grouping_given && notation == Notation::Compact {
        f.grouping = Some(GroupingStrategy::Min2);
    }
    Ok(f)
}

/// The locale list's first tag, or the default (shared with the collator).
fn intl_locale(spec: &str) -> Result<Locale, String> {
    super::intl::resolve_locale(spec)
}

fn id_for(locales: &str, options: &str) -> Result<usize, String> {
    let key = (locales.to_string(), options.to_string());
    if let Some(id) = IDS.with(|m| m.borrow().get(&key).copied()) {
        return Ok(id);
    }
    let f = Rc::new(build(locales, options)?);
    let id = FORMATS.with(|v| {
        let mut v = v.borrow_mut();
        v.push(f);
        v.len() - 1
    });
    IDS.with(|m| m.borrow_mut().insert(key, id));
    Ok(id)
}

/// Rounds and pads a decimal to the digit options.
fn apply_digits(f: &NumFmt, d: &mut Decimal, compact_default: bool) {
    // Multiplying by a power of ten leaves the old digit range behind.
    d.trim_start();
    if let Some((lo, hi)) = f.significant {
        round_significant(d, lo, hi, f.mode);
    } else if let Some((lo, hi)) = f.fraction {
        d.round_with_mode(-hi, f.mode);
        d.trim_end();
        d.pad_end(-lo);
    } else if compact_default {
        // Compact notation without digit options: whole numbers from two integer digits, else two
        // significant digits.
        if !d.is_zero() && d.nonzero_magnitude_start() >= 1 {
            d.round_with_mode(0, f.mode);
            d.trim_end();
        } else {
            round_significant(d, 1, 2, f.mode);
            d.trim_end();
        }
    } else {
        let (lo, hi) = f.default_fraction;
        d.round_with_mode(-hi, f.mode);
        d.trim_end();
        d.pad_end(-lo);
    }
    d.trim_start();
    d.pad_start(f.min_int);
    if f.strip_zeros {
        d.trim_end_if_integer();
    }
}

fn round_significant(d: &mut Decimal, lo: i16, hi: i16, mode: SignedRoundingMode) {
    if d.is_zero() {
        d.pad_end(-(lo - 1));
        return;
    }
    let top = d.nonzero_magnitude_start();
    d.round_with_mode(top - hi + 1, mode);
    d.trim_end();
    let top = d.nonzero_magnitude_start();
    d.pad_end(top - lo + 1);
}

fn grouping_opts(f: &NumFmt) -> DecimalFormatterOptions {
    let mut o = DecimalFormatterOptions::default();
    o.grouping_strategy = f.grouping;
    o
}

fn decimal_formatter(f: &NumFmt) -> Result<DecimalFormatter, String> {
    DecimalFormatter::try_new((&f.locale).into(), grouping_opts(f)).map_err(|e| e.to_string())
}

/// What the locale writes for NaN, where it is not "NaN" (ICU4X does not expose the symbols).
fn nan_text(locale: &Locale) -> &'static str {
    match locale.id.language.as_str() {
        "ar" => "ليس\u{a0}رقم\u{64b}ا",
        "ru" => "не\u{a0}число",
        _ => "NaN",
    }
}

/// The locale's digit for 7 and 0 and its decimal separator, read from a formatted 7.0 and 0.
fn digit_info(f: &NumFmt) -> Result<(char, char, String), String> {
    let mut o = DecimalFormatterOptions::default();
    o.grouping_strategy = Some(GroupingStrategy::Never);
    let fmt = DecimalFormatter::try_new((&f.locale).into(), o).map_err(|e| e.to_string())?;
    let mut seven_five = Decimal::from(7);
    seven_five.multiply_pow10(1);
    seven_five = Decimal::try_from_str("7.5").map_err(|e| format!("{e:?}"))?;
    let s: Vec<char> = fmt.format_to_string(&seven_five).chars().collect();
    let zero = fmt
        .format_to_string(&Decimal::from(0))
        .chars()
        .next()
        .unwrap_or('0');
    let sep: String = s[1..s.len() - 1].iter().collect();
    Ok((s[0], zero, sep))
}

/// Puts `number` (already formatted) where the placeholder 7 was in `shell`, a string the style's
/// formatter produced for the placeholder: the run of the digit 7, the decimal separator and zeros.
fn substitute(shell: &str, number: &str, seven: char, zero: char, sep: &str) -> String {
    let chars: Vec<char> = shell.chars().collect();
    let Some(start) = chars.iter().position(|c| *c == seven) else {
        return shell.to_string();
    };
    let mut end = start + 1;
    let sep_chars: Vec<char> = sep.chars().collect();
    loop {
        if chars.get(end) == Some(&zero) {
            end += 1;
        } else if !sep_chars.is_empty()
            && chars[end..].starts_with(&sep_chars)
            && chars.get(end + sep_chars.len()) == Some(&zero)
        {
            end += sep_chars.len();
        } else {
            break;
        }
    }
    let head: String = chars[..start].iter().collect();
    let tail: String = chars[end..].iter().collect();
    format!("{head}{number}{tail}")
}

/// Formats an unsigned-looking placeholder in the style's formatter and puts `number` in place of
/// its digits. The sign, currency symbol, percent sign and patterns are the formatter's.
fn styled(f: &NumFmt, negative: bool, sign: SignDisplay, number: &str) -> Result<String, String> {
    let (seven, zero, sep) = digit_info(f)?;
    let mut ph = Decimal::from(7);
    if negative {
        ph.set_sign(fixed_decimal::Sign::Negative);
    }
    ph.apply_sign_display(sign);
    let shell = match f.style {
        Style::Decimal => decimal_formatter(f)?.format_to_string(&ph),
        Style::Percent => {
            let fmt = PercentFormatter::try_new_with_decimal_formatter(
                (&f.locale).into(),
                decimal_formatter(f)?,
                Default::default(),
            )
            .map_err(|e| e.to_string())?;
            fmt.format(&ph).to_string()
        }
        Style::Currency => currency_formatter(f)?.format_fixed_decimal(&ph).to_string(),
    };
    Ok(substitute(&shell, number, seven, zero, &sep))
}

fn currency_formatter(f: &NumFmt) -> Result<CurrencyFormatter<DecimalFormatter>, String> {
    let code = f.currency.as_deref().unwrap_or("USD");
    let ty: CurrencyType = code
        .parse()
        .map_err(|_| "currency is not a well-formed currency code".to_string())?;
    let mut opts = CurrencyFormatterOptions::default();
    opts.usage = if f.accounting {
        CurrencyUsage::Accounting
    } else {
        CurrencyUsage::Standard
    };
    match f.currency_display.as_str() {
        "code" => CurrencyFormatter::try_new_code((&f.locale).into(), ty, opts),
        "name" => CurrencyFormatter::try_new_name((&f.locale).into(), ty),
        "narrowSymbol" => CurrencyFormatter::try_new_symbol_narrow((&f.locale).into(), ty, opts),
        _ => CurrencyFormatter::try_new_symbol((&f.locale).into(), ty, opts),
    }
    .map_err(|e| e.to_string())
}

fn format(f: &NumFmt, x: f64) -> Result<String, String> {
    if x.is_nan() || x.is_infinite() {
        let sign = if x.is_nan() && f.sign == SignDisplay::ExceptZero {
            SignDisplay::Auto
        } else {
            f.sign
        };
        let text = if x.is_nan() {
            nan_text(&f.locale)
        } else {
            "∞"
        };
        return styled(f, x.is_infinite() && x < 0.0, sign, text);
    }
    let mut d =
        Decimal::try_from_f64(x, FloatPrecision::RoundTrip).map_err(|e| format!("{e:?}"))?;
    if f.style == Style::Percent {
        d.multiply_pow10(2);
    }
    match f.notation {
        Notation::Scientific | Notation::Engineering => return scientific(f, d),
        Notation::Compact => return compact(f, d),
        Notation::Standard => {}
    }
    apply_digits(f, &mut d, false);
    let negative = d.sign() == fixed_decimal::Sign::Negative;
    let mut abs = d.clone();
    abs.set_sign(fixed_decimal::Sign::None);
    let number = decimal_formatter(f)?.format_to_string(&abs);
    // The placeholder is never zero, so the zero cases of the sign display are decided here.
    let sign = match f.sign {
        SignDisplay::ExceptZero | SignDisplay::Negative if d.is_zero() => SignDisplay::Never,
        other => other,
    };
    styled(f, negative, sign, &number)
}

/// Compact notation: pick the exponent from the magnitude, round the mantissa to the digit options
/// (two significant digits or whole numbers by default), then let ICU4X write it.
fn compact(f: &NumFmt, d: Decimal) -> Result<String, String> {
    let mut opts = CompactDecimalFormatterOptions::default();
    opts.grouping_strategy = f.grouping.or(opts.grouping_strategy);
    let fmt = if f.compact_long {
        CompactDecimalFormatter::try_new_long((&f.locale).into(), opts)
    } else {
        CompactDecimalFormatter::try_new_short((&f.locale).into(), opts)
    }
    .map_err(|e| e.to_string())?;
    let default_rule = f.fraction.is_none() && f.significant.is_none();
    let magnitude = if d.is_zero() {
        0
    } else {
        d.nonzero_magnitude_start()
    };
    let mut exp = fmt.compact_exponent_for_magnitude(magnitude);
    let mut m = d.clone();
    m.multiply_pow10(-i16::from(exp));
    apply_digits(f, &mut m, default_rule);
    // Rounding can carry into the next unit (999.9K is 1M).
    if !m.is_zero() {
        let carried =
            fmt.compact_exponent_for_magnitude(m.nonzero_magnitude_start() + i16::from(exp));
        if carried != exp {
            m.multiply_pow10(i16::from(exp) - i16::from(carried));
            exp = carried;
            apply_digits(f, &mut m, default_rule);
        }
    }
    m.apply_sign_display(f.sign);
    fmt.format_with_exponent(&m, exp)
        .map(|w| w.to_string())
        .map_err(|e| e.to_string())
}

/// `1.235E6` / `1.235E3`: a mantissa in the decimal formatter and the exponent after an `E`.
fn scientific(f: &NumFmt, d: Decimal) -> Result<String, String> {
    let mut m = d;
    let mut exp: i16 = if m.is_zero() {
        0
    } else {
        m.nonzero_magnitude_start()
    };
    let step = if f.notation == Notation::Engineering {
        3
    } else {
        1
    };
    exp = exp.div_euclid(step) * step;
    m.multiply_pow10(-exp);
    apply_digits(f, &mut m, false);
    // Rounding can carry into a new digit (9.9996 -> 10.000): normalise again.
    if !m.is_zero() && m.nonzero_magnitude_start() >= step {
        m.multiply_pow10(-step);
        exp += step;
        apply_digits(f, &mut m, false);
    }
    m.apply_sign_display(f.sign);
    let fmt = decimal_formatter(f)?;
    let e = Decimal::from(i64::from(exp));
    Ok(format!(
        "{}E{}",
        fmt.format_to_string(&m),
        fmt.format_to_string(&e)
    ))
}

/// `new Intl.NumberFormat(locales, options)`: an id for `ts_rt_number_format`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_number_format_new(locales: *const Str, options: *const Str) -> f64 {
    // SAFETY: by contract.
    let (l, o) = unsafe { ((*locales).to_string_lossy(), (*options).to_string_lossy()) };
    match id_for(&l, &o) {
        Ok(id) => id as f64,
        Err(m) => {
            raise_for(&m);
            0.0
        }
    }
}

fn raise_for(m: &str) {
    match m.strip_prefix("TypeError: ") {
        Some(t) => raise(2.0, t),
        None => raise(1.0, m),
    }
}

/// The pieces of a formatted number, found from the text: runs of the locale's digits are
/// `integer` before the decimal separator and `fraction` after it, group separators between integer
/// digits are `group`, and the rest is the sign, percent sign, currency, compact suffix or literal.
fn number_parts(f: &NumFmt, x: f64, text: &str) -> Result<Vec<(String, String)>, String> {
    let (_seven, zero, sep) = digit_info(f)?;
    let is_digit = |c: char| (zero..=char::from_u32(zero as u32 + 9).unwrap_or(zero)).contains(&c);
    // The group separator: what a million looks like between its digits.
    let mut o = DecimalFormatterOptions::default();
    o.grouping_strategy = Some(GroupingStrategy::Always);
    let big = DecimalFormatter::try_new((&f.locale).into(), o)
        .map_err(|e| e.to_string())?
        .format_to_string(&Decimal::from(1_000_000));
    let group: String = big
        .chars()
        .skip_while(|c| is_digit(*c))
        .take_while(|c| !is_digit(*c))
        .collect();
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut push = |kind: &str, s: String| match out.last_mut() {
        Some((k, v)) if k == kind && kind == "literal" => v.push_str(&s),
        _ => out.push((kind.to_string(), s)),
    };
    let sep_chars: Vec<char> = sep.chars().collect();
    let group_chars: Vec<char> = group.chars().collect();
    let mut i = 0;
    let mut after_decimal = false;
    let mut in_exponent = false;
    while i < chars.len() {
        let c = chars[i];
        if is_digit(c) {
            let start = i;
            while i < chars.len() && is_digit(chars[i]) {
                i += 1;
            }
            let kind = if in_exponent {
                "exponentInteger"
            } else if after_decimal {
                "fraction"
            } else {
                "integer"
            };
            push(kind, chars[start..i].iter().collect());
            continue;
        }
        if !in_exponent
            && !after_decimal
            && !sep_chars.is_empty()
            && chars[i..].starts_with(&sep_chars)
            && chars.get(i + sep_chars.len()).is_some_and(|d| is_digit(*d))
            && i > 0
            && is_digit(chars[i - 1])
        {
            push("decimal", sep.clone());
            i += sep_chars.len();
            after_decimal = true;
            continue;
        }
        if !group_chars.is_empty()
            && !after_decimal
            && !in_exponent
            && chars[i..].starts_with(&group_chars)
            && chars
                .get(i + group_chars.len())
                .is_some_and(|d| is_digit(*d))
            && i > 0
            && is_digit(chars[i - 1])
        {
            push("group", group.clone());
            i += group_chars.len();
            continue;
        }
        match c {
            '-' | '\u{2212}' => push(
                if in_exponent {
                    "exponentMinusSign"
                } else {
                    "minusSign"
                },
                c.to_string(),
            ),
            '+' => push("plusSign", c.to_string()),
            '%' | '\u{66a}' => push("percentSign", c.to_string()),
            'E' if chars.get(i + 1).is_some_and(|d| is_digit(*d) || *d == '-') => {
                push("exponentSeparator", "E".to_string());
                in_exponent = true;
            }
            '\u{221e}' => push("infinity", c.to_string()),
            c if c.is_whitespace() || c == '\u{a0}' || c == '(' || c == ')' => {
                push("literal", c.to_string())
            }
            _ => {
                // A run of letters or symbols: NaN, the currency or the compact suffix.
                let start = i;
                while i < chars.len()
                    && !is_digit(chars[i])
                    && !chars[i].is_whitespace()
                    && !matches!(
                        chars[i],
                        '-' | '+' | '%' | '(' | ')' | '\u{a0}' | '\u{221e}'
                    )
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let kind = if x.is_nan() {
                    "nan"
                } else if f.notation == Notation::Compact && f.style != Style::Currency {
                    "compact"
                } else if f.style == Style::Currency {
                    "currency"
                } else if f.notation == Notation::Compact {
                    "compact"
                } else {
                    "literal"
                };
                push(kind, word);
                continue;
            }
        }
        i += 1;
    }
    Ok(out)
}

/// `numberFormat.formatToParts(x)` as encoded records.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_number_format_parts_gc(id: f64, x: f64) -> *mut Str {
    let f = FORMATS.with(|v| v.borrow()[id as usize].clone());
    let r = format(&f, x).and_then(|text| number_parts(&f, x, &text));
    match r {
        Ok(parts) => str_from(&super::intl_parts::encode(&parts)),
        Err(m) => {
            raise_for(&m);
            str_from("")
        }
    }
}

fn run(id: usize, x: f64) -> *mut Str {
    let f = FORMATS.with(|v| v.borrow()[id].clone());
    match format(&f, x) {
        Ok(s) => str_from(&s),
        Err(m) => {
            raise_for(&m);
            str_from("")
        }
    }
}

fn str_from(s: &str) -> *mut Str {
    // SAFETY: valid bytes.
    unsafe { ts_rt_str_new_gc(s.as_ptr(), s.len() as i64) }
}

/// `format.format(x)`.
#[unsafe(no_mangle)]
pub extern "C" fn ts_rt_number_format_gc(id: f64, x: f64) -> *mut Str {
    run(id as usize, x)
}

/// `x.toLocaleString(locales, options)`.
///
/// # Safety
/// Both must be live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ts_rt_num_locale_string_gc(
    x: f64,
    locales: *const Str,
    options: *const Str,
) -> *mut Str {
    // SAFETY: by contract.
    let (l, o) = unsafe { ((*locales).to_string_lossy(), (*options).to_string_lossy()) };
    match id_for(&l, &o) {
        Ok(id) => run(id, x),
        Err(m) => {
            raise_for(&m);
            str_from("")
        }
    }
}
