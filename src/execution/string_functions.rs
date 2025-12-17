// String Functions Executor
//
// Optimized execution engine for all SQL Server string functions
// with security validation and performance optimizations

use crate::error::{DbError, Result};
use crate::parser::string_functions::{StringExpr, StringFunction};
use std::collections::HashMap;

/// Maximum string length to prevent DoS attacks (10MB)
const MAX_STRING_LENGTH: usize = 10_485_760;

/// Maximum repetition count for REPLICATE/SPACE
const MAX_REPLICATE_COUNT: usize = 1_000_000;

/// Security validator for string functions
pub struct StringFunctionValidator;

impl StringFunctionValidator {
    /// Validate string length doesn't exceed limits
    pub fn validate_length(s: &str) -> Result<()> {
        if s.len() > MAX_STRING_LENGTH {
            return Err(DbError::InvalidInput(format!(
                "String length {} exceeds maximum allowed length {}",
                s.len(),
                MAX_STRING_LENGTH
            )));
        }
        Ok(())
    }

    /// Validate numeric value is within safe range
    pub fn validate_count(count: i64) -> Result<usize> {
        if count < 0 {
            return Err(DbError::InvalidInput(
                "Count must be non-negative".to_string(),
            ));
        }
        if count as usize > MAX_REPLICATE_COUNT {
            return Err(DbError::InvalidInput(format!(
                "Count {} exceeds maximum allowed {}",
                count, MAX_REPLICATE_COUNT
            )));
        }
        Ok(count as usize)
    }

    /// Validate ASCII/Unicode value
    pub fn validate_char_code(code: i64) -> Result<u32> {
        if code < 0 || code > 0x10FFFF {
            return Err(DbError::InvalidInput(format!(
                "Invalid character code: {}",
                code
            )));
        }
        Ok(code as u32)
    }
}

/// String function executor with optimizations
pub struct StringFunctionExecutor {
    /// Memoization cache for expensive operations
    #[allow(dead_code)]
    soundex_cache: HashMap<String, String>,
}

impl StringFunctionExecutor {
    pub fn new() -> Self {
        Self {
            soundex_cache: HashMap::new(),
        }
    }

    /// Execute a string function
    pub fn execute(
        &mut self,
        func: &StringFunction,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        match func {
            StringFunction::Ascii(expr) => self.exec_ascii(expr, context),
            StringFunction::Char(expr) => self.exec_char(expr, context),
            StringFunction::CharIndex {
                substring,
                string,
                start_position,
            } => self.exec_charindex(substring, string, start_position.as_ref(), context),
            StringFunction::Concat(exprs) => self.exec_concat(exprs, context),
            StringFunction::ConcatWs { separator, strings } => {
                self.exec_concat_ws(separator, strings, context)
            }
            StringFunction::DataLength(expr) => self.exec_datalength(expr, context),
            StringFunction::Difference { string1, string2 } => {
                self.exec_difference(string1, string2, context)
            }
            StringFunction::Format {
                value,
                format,
                culture,
            } => self.exec_format(value, format, culture.as_ref(), context),
            StringFunction::Left { string, length } => self.exec_left(string, length, context),
            StringFunction::Len(expr) => self.exec_len(expr, context),
            StringFunction::Lower(expr) => self.exec_lower(expr, context),
            StringFunction::LTrim(expr) => self.exec_ltrim(expr, context),
            StringFunction::NChar(expr) => self.exec_nchar(expr, context),
            StringFunction::PatIndex { pattern, string } => {
                self.exec_patindex(pattern, string, context)
            }
            StringFunction::QuoteName { string, quote_char } => {
                self.exec_quotename(string, quote_char.as_ref(), context)
            }
            StringFunction::Replace {
                string,
                old_substring,
                new_substring,
            } => self.exec_replace(string, old_substring, new_substring, context),
            StringFunction::Replicate { string, count } => {
                self.exec_replicate(string, count, context)
            }
            StringFunction::Reverse(expr) => self.exec_reverse(expr, context),
            StringFunction::Right { string, length } => self.exec_right(string, length, context),
            StringFunction::RTrim(expr) => self.exec_rtrim(expr, context),
            StringFunction::Soundex(expr) => self.exec_soundex(expr, context),
            StringFunction::Space(expr) => self.exec_space(expr, context),
            StringFunction::Str {
                number,
                length,
                decimals,
            } => self.exec_str(number, length.as_ref(), decimals.as_ref(), context),
            StringFunction::Stuff {
                string,
                start,
                length,
                new_string,
            } => self.exec_stuff(string, start, length, new_string, context),
            StringFunction::Substring {
                string,
                start,
                length,
            } => self.exec_substring(string, start, length, context),
            StringFunction::Translate {
                string,
                characters,
                translations,
            } => self.exec_translate(string, characters, translations, context),
            StringFunction::Trim { string, characters } => {
                self.exec_trim(string, characters.as_ref(), context)
            }
            StringFunction::Unicode(expr) => self.exec_unicode(expr, context),
            StringFunction::Upper(expr) => self.exec_upper(expr, context),
        }
    }

    /// Evaluate a string expression
    fn eval_expr(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        match expr {
            StringExpr::Literal(s) => {
                StringFunctionValidator::validate_length(s)?;
                Ok(s.clone())
            }
            StringExpr::Column(col) => context
                .get(col)
                .cloned()
                .ok_or_else(|| DbError::InvalidInput(format!("Column '{}' not found", col))),
            StringExpr::Function(_func) => {
                // Recursive evaluation - need mutable self
                Err(DbError::NotImplemented("Nested functions".to_string()))
            }
            StringExpr::Integer(i) => Ok(i.to_string()),
            StringExpr::Float(f) => Ok(f.to_string()),
            StringExpr::Parameter(p) => context
                .get(&format!("${}", p))
                .cloned()
                .ok_or_else(|| DbError::InvalidInput(format!("Parameter ${} not found", p))),
        }
    }

    /// Evaluate expression as integer
    fn eval_as_int(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<i64> {
        match expr {
            StringExpr::Integer(i) => Ok(*i),
            _ => {
                let s = self.eval_expr(expr, context)?;
                s.parse::<i64>().map_err(|_| {
                    DbError::InvalidInput(format!("Cannot convert '{}' to integer", s))
                })
            }
        }
    }

    // ========================================================================
    // STRING FUNCTION IMPLEMENTATIONS
    // ========================================================================

    fn exec_ascii(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let s = self.eval_expr(expr, context)?;
        if s.is_empty() {
            Ok("0".to_string())
        } else {
            Ok((s.chars().next().unwrap() as u32).to_string())
        }
    }

    fn exec_char(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let code = self.eval_as_int(expr, context)?;
        let validated = StringFunctionValidator::validate_char_code(code)?;

        if validated > 127 {
            // Extended ASCII - return empty for codes > 127 (SQL Server behavior)
            return Ok(String::new());
        }

        Ok((validated as u8 as char).to_string())
    }

    fn exec_charindex(
        &self,
        substring: &StringExpr,
        string: &StringExpr,
        start_pos: Option<&Box<StringExpr>>,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let substr = self.eval_expr(substring, context)?;
        let s = self.eval_expr(string, context)?;

        let start = if let Some(pos_expr) = start_pos {
            let pos = self.eval_as_int(pos_expr, context)?;
            if pos < 1 {
                return Ok("0".to_string());
            }
            (pos - 1) as usize // SQL Server uses 1-based indexing
        } else {
            0
        };

        if start >= s.len() {
            return Ok("0".to_string());
        }

        match s[start..].find(&substr) {
            Some(pos) => Ok((start + pos + 1).to_string()), // Convert back to 1-based
            None => Ok("0".to_string()),
        }
    }

    fn exec_concat(
        &self,
        exprs: &[StringExpr],
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let mut result = String::new();
        for expr in exprs {
            let s = self.eval_expr(expr, context)?;
            result.push_str(&s);
            StringFunctionValidator::validate_length(&result)?;
        }
        Ok(result)
    }

    fn exec_concat_ws(
        &self,
        separator: &StringExpr,
        strings: &[StringExpr],
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let sep = self.eval_expr(separator, context)?;
        let mut result = String::new();

        for (i, expr) in strings.iter().enumerate() {
            if i > 0 {
                result.push_str(&sep);
            }
            let s = self.eval_expr(expr, context)?;
            result.push_str(&s);
            StringFunctionValidator::validate_length(&result)?;
        }
        Ok(result)
    }

    fn exec_datalength(
        &self,
        expr: &StringExpr,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s = self.eval_expr(expr, context)?;
        Ok(s.len().to_string())
    }

    fn exec_difference(
        &self,
        string1: &StringExpr,
        string2: &StringExpr,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s1 = self.eval_expr(string1, context)?;
        let s2 = self.eval_expr(string2, context)?;

        // DIFFERENCE compares SOUNDEX codes
        let soundex1 = self.soundex_impl(&s1);
        let soundex2 = self.soundex_impl(&s2);

        // Count matching characters (0-4)
        let matches = soundex1
            .chars()
            .zip(soundex2.chars())
            .filter(|(a, b)| a == b)
            .count();

        Ok(matches.to_string())
    }

    fn exec_format(
        &self,
        value: &StringExpr,
        format: &StringExpr,
        _culture: Option<&Box<StringExpr>>,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let val = self.eval_expr(value, context)?;
        let fmt = self.eval_expr(format, context)?;

        // Basic format implementation
        // In production, this would support complex format strings
        match fmt.as_str() {
            "C" | "c" => Ok(format!("${}", val)), // Currency
            "D" | "d" => Ok(val),                 // Decimal
            "N" | "n" => {
                // Number with commas (thousands separator)
                if let Ok(num) = val.parse::<i64>() {
                    // FIXED: Implement proper thousands separator formatting
                    Ok(Self::format_number_with_thousands_separator(num))
                } else if let Ok(num) = val.parse::<f64>() {
                    Ok(Self::format_float_with_thousands_separator(num))
                } else {
                    Ok(val)
                }
            }
            _ => Ok(val), // Passthrough for unknown formats
        }
    }

    fn exec_left(
        &self,
        string: &StringExpr,
        length: &StringExpr,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s = self.eval_expr(string, context)?;
        let len = self.eval_as_int(length, context)?;

        if len < 0 {
            return Ok(String::new());
        }

        let len = len as usize;
        Ok(s.chars().take(len).collect())
    }

    fn exec_len(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let s = self.eval_expr(expr, context)?;
        // LEN returns length without trailing spaces
        Ok(s.trim_end().len().to_string())
    }

    fn exec_lower(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let s = self.eval_expr(expr, context)?;
        Ok(s.to_lowercase())
    }

    fn exec_ltrim(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let s = self.eval_expr(expr, context)?;
        Ok(s.trim_start().to_string())
    }

    fn exec_nchar(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let code = self.eval_as_int(expr, context)?;
        let validated = StringFunctionValidator::validate_char_code(code)?;

        if let Some(ch) = char::from_u32(validated) {
            Ok(ch.to_string())
        } else {
            Err(DbError::InvalidInput(format!(
                "Invalid Unicode code point: {}",
                code
            )))
        }
    }

    fn exec_patindex(
        &self,
        pattern: &StringExpr,
        string: &StringExpr,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let pat = self.eval_expr(pattern, context)?;
        let s = self.eval_expr(string, context)?;

        // Convert SQL pattern to regex (simplified)
        let _regex_pattern = pat
            .replace('%', ".*")
            .replace('_', ".")
            .replace('[', "\\[")
            .replace(']', "\\]");

        // Simple pattern matching (production would use regex crate)
        if let Some(pos) = s.find(&pat.trim_matches('%').trim_matches('_')) {
            Ok((pos + 1).to_string())
        } else {
            Ok("0".to_string())
        }
    }

    fn exec_quotename(
        &self,
        string: &StringExpr,
        quote_char: Option<&Box<StringExpr>>,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s = self.eval_expr(string, context)?;
        let quote = if let Some(qc_expr) = quote_char {
            let qc = self.eval_expr(qc_expr, context)?;
            qc.chars().next().unwrap_or('[')
        } else {
            '['
        };

        let close_quote = match quote {
            '[' => ']',
            '\'' => '\'',
            '"' => '"',
            _ => quote,
        };

        // Escape internal quotes
        let escaped = s.replace(close_quote, &format!("{}{}", close_quote, close_quote));
        Ok(format!("{}{}{}", quote, escaped, close_quote))
    }

    fn exec_replace(
        &self,
        string: &StringExpr,
        old_substring: &StringExpr,
        new_substring: &StringExpr,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s = self.eval_expr(string, context)?;
        let old = self.eval_expr(old_substring, context)?;
        let new = self.eval_expr(new_substring, context)?;

        let result = s.replace(&old, &new);
        StringFunctionValidator::validate_length(&result)?;
        Ok(result)
    }

    fn exec_replicate(
        &self,
        string: &StringExpr,
        count: &StringExpr,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s = self.eval_expr(string, context)?;
        let n = self.eval_as_int(count, context)?;
        let validated_count = StringFunctionValidator::validate_count(n)?;

        let result = s.repeat(validated_count);
        StringFunctionValidator::validate_length(&result)?;
        Ok(result)
    }

    fn exec_reverse(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let s = self.eval_expr(expr, context)?;
        Ok(s.chars().rev().collect())
    }

    fn exec_right(
        &self,
        string: &StringExpr,
        length: &StringExpr,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s = self.eval_expr(string, context)?;
        let len = self.eval_as_int(length, context)?;

        if len < 0 {
            return Ok(String::new());
        }

        let len = len as usize;
        let chars: Vec<char> = s.chars().collect();
        let start = chars.len().saturating_sub(len);
        Ok(chars[start..].iter().collect())
    }

    fn exec_rtrim(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let s = self.eval_expr(expr, context)?;
        Ok(s.trim_end().to_string())
    }

    fn exec_soundex(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let s = self.eval_expr(expr, context)?;
        Ok(self.soundex_impl(&s))
    }

    /// SOUNDEX algorithm implementation
    fn soundex_impl(&self, s: &str) -> String {
        if s.is_empty() {
            return "0000".to_string();
        }

        let s = s.to_uppercase();
        let mut chars = s.chars();
        let first = chars.next().unwrap();

        if !first.is_alphabetic() {
            return "0000".to_string();
        }

        let mut code = vec![first];
        let mut prev_digit = Self::soundex_digit(first);

        for ch in chars {
            if code.len() >= 4 {
                break;
            }

            let digit = Self::soundex_digit(ch);
            if digit != '0' && digit != prev_digit {
                code.push(digit);
                prev_digit = digit;
            } else if digit == '0' {
                prev_digit = '0';
            }
        }

        // Pad with zeros
        while code.len() < 4 {
            code.push('0');
        }

        code.iter().collect()
    }

    fn soundex_digit(ch: char) -> char {
        match ch.to_ascii_uppercase() {
            'B' | 'F' | 'P' | 'V' => '1',
            'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2',
            'D' | 'T' => '3',
            'L' => '4',
            'M' | 'N' => '5',
            'R' => '6',
            _ => '0',
        }
    }

    fn exec_space(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let n = self.eval_as_int(expr, context)?;
        let validated_count = StringFunctionValidator::validate_count(n)?;
        Ok(" ".repeat(validated_count))
    }

    fn exec_str(
        &self,
        number: &StringExpr,
        length: Option<&Box<StringExpr>>,
        decimals: Option<&Box<StringExpr>>,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let num_str = self.eval_expr(number, context)?;
        let num: f64 = num_str.parse().map_err(|_| {
            DbError::InvalidInput(format!("Cannot convert '{}' to number", num_str))
        })?;

        let total_len = if let Some(len_expr) = length {
            self.eval_as_int(len_expr, context)? as usize
        } else {
            10
        };

        let dec_places = if let Some(dec_expr) = decimals {
            self.eval_as_int(dec_expr, context)? as usize
        } else {
            0
        };

        let formatted = if dec_places > 0 {
            format!("{:.prec$}", num, prec = dec_places)
        } else {
            format!("{:.0}", num)
        };

        // Right-align with spaces
        Ok(format!("{:>width$}", formatted, width = total_len))
    }

    fn exec_stuff(
        &self,
        string: &StringExpr,
        start: &StringExpr,
        length: &StringExpr,
        new_string: &StringExpr,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s = self.eval_expr(string, context)?;
        let start_pos = self.eval_as_int(start, context)?;
        let len = self.eval_as_int(length, context)?;
        let new_s = self.eval_expr(new_string, context)?;

        if start_pos < 1 {
            return Err(DbError::InvalidInput(
                "Start position must be >= 1".to_string(),
            ));
        }

        let start_idx = (start_pos - 1) as usize;
        let chars: Vec<char> = s.chars().collect();

        if start_idx > chars.len() {
            return Ok(s);
        }

        let end_idx = (start_idx + len as usize).min(chars.len());

        let mut result = String::new();
        result.push_str(&chars[..start_idx].iter().collect::<String>());
        result.push_str(&new_s);
        result.push_str(&chars[end_idx..].iter().collect::<String>());

        StringFunctionValidator::validate_length(&result)?;
        Ok(result)
    }

    fn exec_substring(
        &self,
        string: &StringExpr,
        start: &StringExpr,
        length: &StringExpr,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s = self.eval_expr(string, context)?;
        let start_pos = self.eval_as_int(start, context)?;
        let len = self.eval_as_int(length, context)?;

        if start_pos < 1 || len < 0 {
            return Ok(String::new());
        }

        let start_idx = (start_pos - 1) as usize;
        let chars: Vec<char> = s.chars().collect();

        if start_idx >= chars.len() {
            return Ok(String::new());
        }

        let end_idx = (start_idx + len as usize).min(chars.len());
        Ok(chars[start_idx..end_idx].iter().collect())
    }

    fn exec_translate(
        &self,
        string: &StringExpr,
        characters: &StringExpr,
        translations: &StringExpr,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s = self.eval_expr(string, context)?;
        let chars_to_replace = self.eval_expr(characters, context)?;
        let replacements = self.eval_expr(translations, context)?;

        let char_vec: Vec<char> = chars_to_replace.chars().collect();
        let repl_vec: Vec<char> = replacements.chars().collect();

        let mut result = s.clone();
        for (i, &ch) in char_vec.iter().enumerate() {
            if i < repl_vec.len() {
                result = result.replace(ch, &repl_vec[i].to_string());
            } else {
                // If no corresponding translation, remove the character
                result = result.replace(ch, "");
            }
        }

        Ok(result)
    }

    fn exec_trim(
        &self,
        string: &StringExpr,
        characters: Option<&Box<StringExpr>>,
        context: &HashMap<String, String>,
    ) -> Result<String> {
        let s = self.eval_expr(string, context)?;

        if let Some(chars_expr) = characters {
            let chars_to_trim = self.eval_expr(chars_expr, context)?;
            let trim_chars: Vec<char> = chars_to_trim.chars().collect();
            Ok(s.trim_matches(|c| trim_chars.contains(&c)).to_string())
        } else {
            Ok(s.trim().to_string())
        }
    }

    fn exec_unicode(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let s = self.eval_expr(expr, context)?;
        if s.is_empty() {
            Ok("0".to_string())
        } else {
            Ok((s.chars().next().unwrap() as u32).to_string())
        }
    }

    fn exec_upper(&self, expr: &StringExpr, context: &HashMap<String, String>) -> Result<String> {
        let s = self.eval_expr(expr, context)?;
        Ok(s.to_uppercase())
    }

    /// Format integer with thousands separator (commas)
    /// Example: 1234567 => "1,234,567"
    fn format_number_with_thousands_separator(num: i64) -> String {
        let is_negative = num < 0;
        let abs_num = num.abs();
        let num_str = abs_num.to_string();

        let mut result = String::new();
        let len = num_str.len();

        for (i, ch) in num_str.chars().enumerate() {
            result.push(ch);
            let remaining = len - i - 1;
            if remaining > 0 && remaining % 3 == 0 {
                result.push(',');
            }
        }

        if is_negative {
            format!("-{}", result)
        } else {
            result
        }
    }

    /// Format float with thousands separator (commas)
    /// Example: 1234567.89 => "1,234,567.89"
    fn format_float_with_thousands_separator(num: f64) -> String {
        let is_negative = num < 0.0;
        let abs_num = num.abs();

        // Split into integer and fractional parts
        let integer_part = abs_num.trunc() as i64;
        let fractional_part = abs_num.fract();

        // Format integer part with commas
        let integer_str = Self::format_number_with_thousands_separator(integer_part);

        // Format fractional part
        if fractional_part > 0.0 {
            let frac_formatted = format!("{:.6}", fractional_part);
            let frac_str = frac_formatted
                .trim_start_matches("0.")
                .trim_end_matches('0');

            if is_negative && integer_part == 0 {
                format!("-{}.{}", integer_str.trim_start_matches('-'), frac_str)
            } else if frac_str.is_empty() {
                integer_str
            } else {
                format!("{}.{}", integer_str.trim_start_matches('-'), frac_str)
            }
        } else {
            if is_negative && integer_part == 0 {
                format!("-{}", integer_str)
            } else {
                integer_str
            }
        }
    }
}

impl Default for StringFunctionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upper_lower() {
        let mut executor = StringFunctionExecutor::new();
        let context = HashMap::new();

        let result = executor
            .exec_upper(&StringExpr::Literal("hello".to_string()), &context)
            .unwrap();
        assert_eq!(result, "HELLO");

        let result = executor
            .exec_lower(&StringExpr::Literal("WORLD".to_string()), &context)
            .unwrap();
        assert_eq!(result, "world");
    }

    #[test]
    fn test_soundex() {
        let executor = StringFunctionExecutor::new();
        assert_eq!(executor.soundex_impl("Robert"), "R163");
        assert_eq!(executor.soundex_impl("Rupert"), "R163");
        assert_eq!(executor.soundex_impl("Smith"), "S530");
    }

    #[test]
    fn test_security_validation() {
        assert!(StringFunctionValidator::validate_count(-1).is_err());
        assert!(StringFunctionValidator::validate_count(2000000).is_err());
        assert!(StringFunctionValidator::validate_count(100).is_ok());
    }
}
