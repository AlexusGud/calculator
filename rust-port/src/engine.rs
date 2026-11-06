use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CalculatorMode {
    Standard = 0,
    Scientific = 1,
    Programmer = 2,
}

pub static PROGRAMMER_WORD_SIZE: AtomicU32 = AtomicU32::new(64);
pub static ANGLES_IN_DEGREES: AtomicBool = AtomicBool::new(true);
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Identifier(String),
    Operator(String),
    LeftParen,
    RightParen,
    Comma,
    Percent,
    Factorial,
}

struct ExpressionParser {
    tokens: Vec<Token>,
    position: usize,
    mode: CalculatorMode,
}

pub fn parse_and_calculate(
    expression: &str,
    mode: CalculatorMode,
    radix: u32,
) -> Result<f64, String> {
    let tokens = tokenize(expression, mode, radix)?;
    let mut parser = ExpressionParser {
        tokens,
        position: 0,
        mode,
    };
    let value = parser.parse_expression(0)?;
    if parser.position != parser.tokens.len() {
        return Err("Unexpected input after expression".to_string());
    }
    if !value.is_finite() {
        return Err("Result is outside the supported numeric range".to_string());
    }
    if mode == CalculatorMode::Programmer {
        let integer = as_i64(value)?;
        return Ok(
            (integer as u64 & word_mask(PROGRAMMER_WORD_SIZE.load(Ordering::Acquire))) as i64
                as f64,
        );
    }
    Ok(value)
}

fn tokenize(expression: &str, mode: CalculatorMode, radix: u32) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = expression.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < chars.len() {
        let current = chars[index];
        if current.is_whitespace() {
            index += 1;
            continue;
        }

        if current.is_ascii_digit()
            || (mode == CalculatorMode::Programmer
                && radix == 16
                && current.is_ascii_hexdigit()
                && current.is_ascii_alphabetic())
        {
            let start = index;
            index += 1;
            while index < chars.len() {
                let ch = chars[index];
                let valid_digit = ch.to_digit(radix).is_some();
                let decimal_point = mode != CalculatorMode::Programmer && ch == '.';
                let exponent_part = mode != CalculatorMode::Programmer
                    && matches!(ch, 'e' | 'E')
                    && index + 1 < chars.len();
                if valid_digit || decimal_point || exponent_part {
                    index += 1;
                    if exponent_part && index < chars.len() && matches!(chars[index], '+' | '-') {
                        index += 1;
                    }
                } else {
                    break;
                }
            }

            let number: String = chars[start..index].iter().collect();
            let value = if mode == CalculatorMode::Programmer {
                i64::from_str_radix(&number, radix)
                    .map(|number| number as f64)
                    .map_err(|_| format!("Invalid base-{radix} integer: {number}"))?
            } else {
                number
                    .parse::<f64>()
                    .map_err(|_| format!("Invalid number: {number}"))?
            };
            tokens.push(Token::Number(value));
            continue;
        }

        if current.is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < chars.len() && chars[index].is_ascii_alphabetic() {
                index += 1;
            }
            let name: String = chars[start..index]
                .iter()
                .collect::<String>()
                .to_ascii_lowercase();
            match name.as_str() {
                "and" => tokens.push(Token::Operator("&".to_string())),
                "or" => tokens.push(Token::Operator("|".to_string())),
                "xor" => tokens.push(Token::Operator("xor".to_string())),
                _ => tokens.push(Token::Identifier(name)),
            }
            continue;
        }

        match current {
            '(' => tokens.push(Token::LeftParen),
            ')' => tokens.push(Token::RightParen),
            ',' => tokens.push(Token::Comma),
            '%' => tokens.push(Token::Percent),
            '!' => tokens.push(Token::Factorial),
            '+' | '-' | '*' | '/' | '^' | '&' | '|' => {
                tokens.push(Token::Operator(current.to_string()))
            }
            '<' | '>' if index + 1 < chars.len() && chars[index + 1] == current => {
                tokens.push(Token::Operator(if current == '<' {
                    "<<".to_string()
                } else {
                    ">>".to_string()
                }));
                index += 1;
            }
            'π' => tokens.push(Token::Identifier("pi".to_string())),
            _ => return Err(format!("Unsupported character: {current}")),
        }
        index += 1;
    }

    if tokens.is_empty() {
        return Err("Enter an expression".to_string());
    }
    Ok(tokens)
}

impl ExpressionParser {
    fn parse_expression(&mut self, min_precedence: u8) -> Result<f64, String> {
        let mut left = self.parse_prefix()?;

        loop {
            if self.tokens.get(self.position) == Some(&Token::Percent) {
                self.position += 1;
                left /= 100.0;
                continue;
            }
            if self.tokens.get(self.position) == Some(&Token::Factorial) {
                self.position += 1;
                left = factorial(left)?;
                continue;
            }

            let Some(Token::Operator(operator)) = self.tokens.get(self.position) else {
                break;
            };
            let operator = operator.clone();
            let Some((precedence, right_associative)) = operator_precedence(&operator, self.mode)
            else {
                break;
            };
            if precedence < min_precedence {
                break;
            }
            self.position += 1;
            let right = self.parse_expression(if right_associative {
                precedence
            } else {
                precedence + 1
            })?;
            left = apply_operator(&operator, left, right, self.mode)?;
        }

        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<f64, String> {
        let token = self
            .tokens
            .get(self.position)
            .ok_or_else(|| "Expected a number or expression".to_string())?
            .clone();
        self.position += 1;
        match token {
            Token::Number(value) => Ok(value),
            Token::Operator(operator) if operator == "-" => Ok(-self.parse_expression(8)?),
            Token::Operator(operator) if operator == "+" => self.parse_expression(8),
            Token::LeftParen => {
                let value = self.parse_expression(0)?;
                if self.tokens.get(self.position) != Some(&Token::RightParen) {
                    return Err("Missing closing parenthesis".to_string());
                }
                self.position += 1;
                Ok(value)
            }
            Token::Identifier(name) if name == "pi" => Ok(std::f64::consts::PI),
            Token::Identifier(name) if name == "days" => {
                if self.tokens.get(self.position) != Some(&Token::LeftParen) {
                    return Err("Expected '(' after days".to_string());
                }
                self.position += 1;
                let start_date = self.parse_expression(0)?;
                if self.tokens.get(self.position) != Some(&Token::Comma) {
                    return Err("Date difference needs two YYYYMMDD dates".to_string());
                }
                self.position += 1;
                let end_date = self.parse_expression(0)?;
                if self.tokens.get(self.position) != Some(&Token::RightParen) {
                    return Err("Missing closing parenthesis".to_string());
                }
                self.position += 1;
                date_difference(start_date, end_date)
            }
            Token::Identifier(name) => {
                if self.tokens.get(self.position) != Some(&Token::LeftParen) {
                    return Err(format!("Expected '(' after {name}"));
                }
                self.position += 1;
                let argument = self.parse_expression(0)?;
                if self.tokens.get(self.position) != Some(&Token::RightParen) {
                    return Err("Missing closing parenthesis".to_string());
                }
                self.position += 1;
                apply_function(&name, argument)
            }
            _ => Err("Expected a number or expression".to_string()),
        }
    }
}

fn operator_precedence(operator: &str, mode: CalculatorMode) -> Option<(u8, bool)> {
    if mode == CalculatorMode::Programmer {
        return match operator {
            "|" => Some((1, false)),
            "xor" | "^" => Some((2, false)),
            "&" => Some((3, false)),
            "<<" | ">>" => Some((4, false)),
            "+" | "-" => Some((5, false)),
            "*" | "/" => Some((6, false)),
            _ => None,
        };
    }
    match operator {
        "+" | "-" => Some((1, false)),
        "*" | "/" => Some((2, false)),
        "^" => Some((3, true)),
        _ => None,
    }
}

fn word_mask(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

fn factorial(value: f64) -> Result<f64, String> {
    if value < 0.0 || value.fract() != 0.0 {
        return Err("Factorial requires a non-negative integer".to_string());
    }
    if value > 170.0 {
        return Err("Factorial result is outside the supported numeric range".to_string());
    }
    Ok((2..=value as u64).fold(1.0, |result, factor| result * factor as f64))
}

fn date_difference(start: f64, end: f64) -> Result<f64, String> {
    fn date_serial(date: f64) -> Result<i64, String> {
        if date.fract() != 0.0 || !(10_101.0..=99_991_231.0).contains(&date) {
            return Err("Dates must use YYYYMMDD format".to_string());
        }
        let packed = date as i64;
        let year = packed / 10_000;
        let month = (packed / 100) % 100;
        let day = packed % 100;
        if !(1..=12).contains(&month) {
            return Err("Date month must be from 01 to 12".to_string());
        }
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days_in_month = match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        if day < 1 || day > days_in_month {
            return Err("Invalid day for the selected month".to_string());
        }

        let adjusted_year = year - i64::from(month <= 2);
        let era = adjusted_year.div_euclid(400);
        let year_of_era = adjusted_year - era * 400;
        let adjusted_month = month + if month > 2 { -3 } else { 9 };
        let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        Ok(era * 146_097 + day_of_era)
    }

    Ok((date_serial(end)? - date_serial(start)?) as f64)
}

fn apply_operator(
    operator: &str,
    left: f64,
    right: f64,
    mode: CalculatorMode,
) -> Result<f64, String> {
    if mode == CalculatorMode::Programmer {
        let left = as_i64(left)?;
        let right = as_i64(right)?;
        let value = match operator {
            "+" => left.checked_add(right),
            "-" => left.checked_sub(right),
            "*" => left.checked_mul(right),
            "/" if right != 0 => left.checked_div(right),
            "/" => return Err("Cannot divide by zero".to_string()),
            "|" => Some(left | right),
            "xor" | "^" => Some(left ^ right),
            "&" => Some(left & right),
            "<<" if (0..PROGRAMMER_WORD_SIZE.load(Ordering::Acquire) as i64).contains(&right) => {
                Some(left.wrapping_shl(right as u32))
            }
            ">>" if (0..PROGRAMMER_WORD_SIZE.load(Ordering::Acquire) as i64).contains(&right) => {
                Some(left >> right)
            }
            "<<" | ">>" => {
                return Err("Shift amount must be smaller than the selected word size".to_string());
            }
            _ => return Err(format!("Operator '{operator}' is unavailable in this mode")),
        }
        .ok_or_else(|| "Integer overflow".to_string())?;
        let value = (value as u64 & word_mask(PROGRAMMER_WORD_SIZE.load(Ordering::Acquire))) as i64;
        return Ok(value as f64);
    }

    let value = match operator {
        "+" => left + right,
        "-" => left - right,
        "*" => left * right,
        "/" if right != 0.0 => left / right,
        "/" => return Err("Cannot divide by zero".to_string()),
        "^" => left.powf(right),
        _ => return Err(format!("Operator '{operator}' is unavailable in this mode")),
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err("Result is outside the supported numeric range".to_string())
    }
}

fn as_i64(value: f64) -> Result<i64, String> {
    if !value.is_finite()
        || value.fract() != 0.0
        || value.abs() > 9_007_199_254_740_991.0
        || value < i64::MIN as f64
        || value >= i64::MAX as f64
    {
        return Err(
            "Programmer operations require integers within the exact numeric range".to_string(),
        );
    }
    Ok(value as i64)
}

pub fn apply_function(name: &str, argument: f64) -> Result<f64, String> {
    let angle = if ANGLES_IN_DEGREES.load(Ordering::Acquire) {
        argument.to_radians()
    } else {
        argument
    };
    let value = match name {
        "sin" => angle.sin(),
        "cos" => angle.cos(),
        "tan" => angle.tan(),
        "asin" if (-1.0..=1.0).contains(&argument) => {
            let result = argument.asin();
            if ANGLES_IN_DEGREES.load(Ordering::Acquire) {
                result.to_degrees()
            } else {
                result
            }
        }
        "acos" if (-1.0..=1.0).contains(&argument) => {
            let result = argument.acos();
            if ANGLES_IN_DEGREES.load(Ordering::Acquire) {
                result.to_degrees()
            } else {
                result
            }
        }
        "atan" => {
            let result = argument.atan();
            if ANGLES_IN_DEGREES.load(Ordering::Acquire) {
                result.to_degrees()
            } else {
                result
            }
        }
        "asin" | "acos" => {
            return Err("Inverse trigonometric input must be from -1 to 1".to_string());
        }
        "sinh" => argument.sinh(),
        "cosh" => argument.cosh(),
        "tanh" => argument.tanh(),
        "sqrt" if argument >= 0.0 => argument.sqrt(),
        "sqrt" => return Err("Square root requires a non-negative value".to_string()),
        "log" if argument > 0.0 => argument.log10(),
        "log" => return Err("Logarithm requires a positive value".to_string()),
        "ln" if argument > 0.0 => argument.ln(),
        "ln" => return Err("Natural logarithm requires a positive value".to_string()),
        _ => return Err(format!("Unknown function: {name}")),
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err("Result is outside the supported numeric range".to_string())
    }
}

pub fn format_result(value: f64, mode: CalculatorMode, radix: u32) -> String {
    if mode != CalculatorMode::Programmer {
        return value.to_string();
    }

    let integer = value as i64;
    let encoded = integer as u64 & word_mask(PROGRAMMER_WORD_SIZE.load(Ordering::Acquire));
    match radix {
        2 => format!("{encoded:b}"),
        8 => format!("{encoded:o}"),
        10 => integer.to_string(),
        16 => format!("{encoded:X}"),
        _ => unreachable!("unsupported programmer base"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluates_arithmetic_with_precedence_and_parentheses() {
        assert_eq!(
            parse_and_calculate("2 + 3 * (4 - 1)", CalculatorMode::Standard, 10).unwrap(),
            11.0
        );
    }

    #[test]
    fn evaluates_scientific_functions_and_percent() {
        assert!(
            (parse_and_calculate("sin(30)", CalculatorMode::Scientific, 10).unwrap() - 0.5).abs()
                < 1e-10
        );
        assert_eq!(
            parse_and_calculate("200 * 10%", CalculatorMode::Scientific, 10).unwrap(),
            20.0
        );
        assert_eq!(
            parse_and_calculate("5!", CalculatorMode::Scientific, 10).unwrap(),
            120.0
        );
        assert!(
            (parse_and_calculate("asin(0.5)", CalculatorMode::Scientific, 10).unwrap() - 30.0)
                .abs()
                < 1e-10
        );
        assert_eq!(
            parse_and_calculate("2 ^ 3", CalculatorMode::Scientific, 10).unwrap(),
            8.0
        );
    }

    #[test]
    fn evaluates_programmer_radix_and_bitwise_operations() {
        assert_eq!(
            parse_and_calculate("FF & 0F", CalculatorMode::Programmer, 16).unwrap(),
            15.0
        );
        assert_eq!(
            format_result(
                parse_and_calculate("1010 << 1", CalculatorMode::Programmer, 2).unwrap(),
                CalculatorMode::Programmer,
                2
            ),
            "10100"
        );
    }

    #[test]
    fn reports_invalid_expressions_and_division_by_zero() {
        assert!(parse_and_calculate("1 / 0", CalculatorMode::Standard, 10).is_err());
        assert!(parse_and_calculate("(2 + 3", CalculatorMode::Standard, 10).is_err());
    }

    #[test]
    fn calculates_date_differences_and_checks_calendar_dates() {
        assert_eq!(
            parse_and_calculate("days(20240228,20240301)", CalculatorMode::Standard, 10).unwrap(),
            2.0
        );
        assert!(
            parse_and_calculate("days(20240230,20240301)", CalculatorMode::Standard, 10).is_err()
        );
    }

    #[test]
    fn supports_programmer_word_masks() {
        assert_eq!(word_mask(8), 0xFF);
        assert_eq!(word_mask(16), 0xFFFF);
        assert_eq!(word_mask(32), 0xFFFF_FFFF);
        assert_eq!(word_mask(64), u64::MAX);
    }
}
