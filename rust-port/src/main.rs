#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod converter;
mod engine;

use std::{cell::RefCell, rc::Rc, sync::atomic::Ordering};

use converter::Category;
use engine::{
    ANGLES_IN_DEGREES, CalculatorMode, PROGRAMMER_WORD_SIZE, apply_function, format_result,
    parse_and_calculate,
};
use slint::{ModelRc, SharedString, VecModel};

slint::include_modules!();

#[derive(Clone)]
struct Calculation {
    expression: String,
    result: String,
}

struct AppState {
    mode: CalculatorMode,
    mode_name: String,
    display: String,
    expression: String,
    status: String,
    memory: f64,
    has_memory: bool,
    history: Vec<Calculation>,
    radix: u32,
    word_size: u32,
    show_history: bool,
    show_memory: bool,
    expanded_nav: bool,
    just_calculated: bool,
    second_functions: bool,
    converter_category: Category,
    converter_from: usize,
    converter_to: usize,
    converter_result: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: CalculatorMode::Standard,
            mode_name: "Standard".into(),
            display: "0".into(),
            expression: String::new(),
            status: "Ready".into(),
            memory: 0.0,
            has_memory: false,
            history: Vec::new(),
            radix: 10,
            word_size: 64,
            show_history: false,
            show_memory: false,
            expanded_nav: false,
            just_calculated: false,
            second_functions: false,
            converter_category: Category::Length,
            converter_from: 1,
            converter_to: 6,
            converter_result: String::new(),
        }
    }
}

impl AppState {
    fn select_mode(&mut self, mode: &str) {
        if mode == "Settings" {
            self.mode_name = "Settings".into();
            self.show_history = false;
            self.show_memory = false;
            self.status = "Application settings".into();
            return;
        }
        if let Some(category) = Category::from_nav_name(mode) {
            self.mode = CalculatorMode::Standard;
            self.mode_name = "Converter".into();
            self.converter_category = category;
            self.converter_from = 0;
            self.converter_to = 1;
            self.show_history = false;
            self.show_memory = false;
            self.status = "Ready".into();
            self.update_conversion();
            return;
        }
        self.mode_name = mode.to_string();
        self.mode = match mode {
            "Scientific" => CalculatorMode::Scientific,
            "Programmer" => CalculatorMode::Programmer,
            _ => CalculatorMode::Standard,
        };
        self.show_history = false;
        self.show_memory = false;
        if self.mode == CalculatorMode::Scientific {
            self.status = if ANGLES_IN_DEGREES.load(Ordering::Acquire) {
                "Degrees".into()
            } else {
                "Radians".into()
            };
        } else if self.mode == CalculatorMode::Programmer {
            self.status = format!("{}-bit | {}", self.word_size, self.radix_name());
        } else {
            self.status = "Ready".into();
        }
        if self.mode_name == "Converter" {
            self.update_conversion();
        }
    }

    fn radix_name(&self) -> &'static str {
        match self.radix {
            2 => "BIN",
            8 => "OCT",
            10 => "DEC",
            16 => "HEX",
            _ => "DEC",
        }
    }

    fn value(&self) -> Result<f64, String> {
        parse_and_calculate(&self.display, self.mode, self.radix)
    }

    fn input(&mut self, input: &str) {
        let starts_new_value = self.just_calculated
            && (input.chars().all(|c| c.is_ascii_digit()) || input == "." || input == "π");
        if starts_new_value || (self.display == "0" && input.chars().all(|c| c.is_ascii_digit())) {
            self.display = input.to_string();
        } else if matches!(
            self.display.as_str(),
            "Cannot divide by zero" | "Enter an expression" | "Invalid expression"
        ) {
            self.display = input.to_string();
        } else {
            self.display.push_str(input);
        }
        self.just_calculated = false;
        self.status = "Ready".into();
        if self.mode_name == "Converter" {
            self.update_conversion();
        }
    }

    fn update_conversion(&mut self) {
        let units = self.converter_category.units();
        let (Some(from), Some(to)) = (units.get(self.converter_from), units.get(self.converter_to))
        else {
            self.status = "Select valid units".into();
            return;
        };
        match parse_and_calculate(&self.display, CalculatorMode::Standard, 10).and_then(|value| {
            converter::convert(
                value,
                self.converter_category,
                self.converter_from,
                self.converter_to,
            )
        }) {
            Ok(value) => {
                self.converter_result = format_result(value, CalculatorMode::Standard, 10);
                self.expression = format!(
                    "{} {} = {} {}",
                    self.display, from.symbol, self.converter_result, to.symbol,
                );
                self.status = "Converted".into();
            }
            Err(error) if self.display == "0" => {
                self.converter_result.clear();
                self.expression.clear();
                self.status = error;
            }
            Err(error) => {
                self.converter_result.clear();
                self.status = error;
            }
        }
    }

    fn sync_display(&mut self, display: String) {
        if self.display == display {
            return;
        }
        self.display = display;
        self.just_calculated = false;
        if self.mode_name == "Converter" {
            self.update_conversion();
        } else {
            self.status = "Ready".into();
        }
    }

    fn cycle_converter_category(&mut self, direction: isize) {
        let categories = Category::all();
        let current = categories
            .iter()
            .position(|category| *category == self.converter_category)
            .unwrap_or(0);
        let next = (current as isize + direction).rem_euclid(categories.len() as isize) as usize;
        self.converter_category = categories[next];
        self.converter_from = 0;
        self.converter_to = 1;
        self.update_conversion();
    }

    fn cycle_converter_unit(&mut self, source: bool, direction: isize) {
        let unit_count = self.converter_category.units().len();
        let selected = if source {
            &mut self.converter_from
        } else {
            &mut self.converter_to
        };
        *selected = (*selected as isize + direction).rem_euclid(unit_count as isize) as usize;
        self.update_conversion();
    }

    fn unary(&mut self, function: &str) {
        match self.value().and_then(|value| match function {
            "square" => Ok(value * value),
            "cube" => Ok(value * value * value),
            "reciprocal" if value != 0.0 => Ok(1.0 / value),
            "reciprocal" => Err("Cannot divide by zero".into()),
            "pow10" => Ok(10.0f64.powf(value)),
            "exp" => Ok(value.exp()),
            "sqrt" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh"
            | "tanh" | "log" | "ln" => apply_function(function, value),
            "factorial" => {
                let result =
                    parse_and_calculate(&format!("({})!", self.display), self.mode, self.radix)?;
                Ok(result)
            }
            "abs" => Ok(value.abs()),
            "not" if self.mode == CalculatorMode::Programmer => {
                let bits = self.word_size.min(64);
                let mask = if bits == 64 {
                    u64::MAX
                } else {
                    (1_u64 << bits) - 1
                };
                Ok((!(value as i64 as u64) & mask) as i64 as f64)
            }
            _ => Err("Unsupported function".into()),
        }) {
            Ok(value) if value.is_finite() => {
                self.expression = format!("{function}({})", self.display);
                self.display = format_result(value, self.mode, self.radix);
                self.status = "Calculated".into();
            }
            Ok(_) => self.status = "Result is outside the supported numeric range".into(),
            Err(error) => self.status = error,
        }
    }

    fn calculate(&mut self) {
        match self.value() {
            Ok(value) => {
                let result = format_result(value, self.mode, self.radix);
                if self.history.len() >= 50 {
                    self.history.remove(0);
                }
                self.history.push(Calculation {
                    expression: self.display.clone(),
                    result: result.clone(),
                });
                self.expression = format!("{} =", self.display);
                self.display = result;
                self.status = "Calculated".into();
                self.just_calculated = true;
            }
            Err(error) => self.status = error,
        }
    }

    fn press(&mut self, action: &str) {
        match action {
            "0" if self.display == "0" => {}
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "." => self.input(action),
            "+" => self.input(" + "),
            "−" => self.input(" - "),
            "×" => self.input(" * "),
            "÷" => self.input(" / "),
            "xʸ" => self.input(" ^ "),
            "AND" => self.input(" & "),
            "OR" => self.input(" | "),
            "XOR" => self.input(" xor "),
            "<<" => self.input(" << "),
            ">>" => self.input(" >> "),
            "A" | "B" | "C" | "D" | "E" | "F" if self.mode == CalculatorMode::Programmer => {
                self.input(action)
            }
            "π" => self.input("π"),
            "(" | ")" => self.input(action),
            "%" => self.input("%"),
            "n!" => self.unary("factorial"),
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "log"
            | "ln" => {
                let name = if self.second_functions {
                    match action {
                        "sin" => "asin",
                        "cos" => "acos",
                        "tan" => "atan",
                        _ => action,
                    }
                } else {
                    action
                };
                self.unary(name);
            }
            "x²" => self.unary("square"),
            "x³" => self.unary("cube"),
            "√x" => self.unary("sqrt"),
            "1/x" | "¹/x" => self.unary("reciprocal"),
            "10ˣ" => self.unary("pow10"),
            "eˣ" => self.unary("exp"),
            "abs" => self.unary("abs"),
            "e" => self.input("2.718281828459045"),
            "2nd" => self.second_functions = !self.second_functions,
            "=" => self.calculate(),
            "C" => {
                self.display = "0".into();
                self.expression.clear();
                self.status = "Ready".into();
                if self.mode_name == "Converter" {
                    self.update_conversion();
                }
            }
            "CE" => {
                let trimmed = self.display.trim_end();
                let last_operator = trimmed.rfind(['+', '-', '*', '/', '&', '|']);
                self.display = match last_operator {
                    Some(index) => trimmed[..=index].to_string(),
                    None => "0".into(),
                };
                self.status = "Ready".into();
            }
            "⌫" => {
                self.display.pop();
                if self.display.is_empty() {
                    self.display = "0".into();
                }
            }
            "±" => {
                if let Ok(value) = self.value() {
                    self.display = format_result(-value, self.mode, self.radix);
                    self.just_calculated = true;
                } else {
                    self.input("-");
                }
            }
            "," => self.input(","),
            "HEX" => self.set_radix(16),
            "DEC" => self.set_radix(10),
            "OCT" => self.set_radix(8),
            "BIN" => self.set_radix(2),
            "word8" | "word16" | "word32" | "word64" => {
                self.word_size = match action {
                    "word8" => 8,
                    "word16" => 16,
                    "word32" => 32,
                    "word64" => 64,
                    _ => unreachable!("word-size action was validated by the match arm"),
                };
                PROGRAMMER_WORD_SIZE.store(self.word_size, Ordering::Release);
                self.status = format!("{}-bit | {}", self.word_size, self.radix_name());
            }
            "NOT" => self.unary("not"),
            "history" => {
                self.show_history = !self.show_history;
                self.show_memory = false;
            }
            "memory-panel" => {
                self.show_memory = !self.show_memory;
                self.show_history = false;
            }
            "nav" => self.expanded_nav = !self.expanded_nav,
            "angle-units" => {
                let degrees = !ANGLES_IN_DEGREES.load(Ordering::Acquire);
                ANGLES_IN_DEGREES.store(degrees, Ordering::Release);
                self.status = if degrees { "Degrees" } else { "Radians" }.into();
            }
            "MC" => {
                self.memory = 0.0;
                self.has_memory = false;
                self.status = "Memory cleared".into();
            }
            "MR" => {
                if self.has_memory {
                    self.display = format_result(self.memory, self.mode, self.radix);
                    self.status = "Memory recalled".into();
                }
            }
            "MS" => match self.value() {
                Ok(value) => {
                    self.memory = value;
                    self.has_memory = true;
                    self.status = "Stored in memory".into();
                }
                Err(error) => self.status = error,
            },
            "M+" | "M-" => match self.value() {
                Ok(value) => {
                    self.memory += if action == "M+" { value } else { -value };
                    self.has_memory = true;
                    self.status = "Memory updated".into();
                }
                Err(error) => self.status = error,
            },
            "clear-history" => {
                self.history.clear();
                self.status = "History cleared".into();
            }
            "date-template" => {
                self.display = "days(20240228,20240301)".into();
                self.status = "Edit dates in YYYYMMDD format".into();
            }
            "m-ft" => self.convert(|v| v * 3.28084),
            "ft-m" => self.convert(|v| v / 3.28084),
            "km-mi" => self.convert(|v| v * 0.6213711922),
            "mi-km" => self.convert(|v| v / 0.6213711922),
            "kg-lb" => self.convert(|v| v * 2.2046226218),
            "lb-kg" => self.convert(|v| v / 2.2046226218),
            "c-f" => self.convert(|v| v * 9.0 / 5.0 + 32.0),
            "f-c" => self.convert(|v| (v - 32.0) * 5.0 / 9.0),
            "converter-category-previous" => self.cycle_converter_category(-1),
            "converter-category-next" => self.cycle_converter_category(1),
            "converter-from-previous" => self.cycle_converter_unit(true, -1),
            "converter-from-next" => self.cycle_converter_unit(true, 1),
            "converter-to-previous" => self.cycle_converter_unit(false, -1),
            "converter-to-next" => self.cycle_converter_unit(false, 1),
            "converter-swap" => {
                std::mem::swap(&mut self.converter_from, &mut self.converter_to);
                self.update_conversion();
            }
            _ => {}
        }
    }

    fn set_radix(&mut self, radix: u32) {
        self.radix = radix;
        self.status = format!("{}-bit | {}", self.word_size, self.radix_name());
    }

    fn convert(&mut self, conversion: impl FnOnce(f64) -> f64) {
        match self.value() {
            Ok(value) => {
                let converted = conversion(value);
                if converted.is_finite() {
                    self.display = format_result(converted, self.mode, self.radix);
                    self.status = "Converted".into();
                } else {
                    self.status = "Conversion result is outside the supported range".into();
                }
            }
            Err(error) => self.status = error,
        }
    }

    fn history_text(&self) -> String {
        if self.history.is_empty() {
            "No history yet".into()
        } else {
            self.history
                .iter()
                .rev()
                .take(12)
                .map(|entry| format!("{}\n= {}", entry.expression, entry.result))
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    }

    fn memory_text(&self) -> String {
        if self.has_memory {
            format_result(self.memory, self.mode, self.radix)
        } else {
            "Memory is empty".into()
        }
    }

    fn programmer_bits(&self) -> String {
        let Ok(value) = self.value() else {
            return String::new();
        };
        let bit_string = format!(
            "{:0width$b}",
            value as i64 as u64,
            width = self.word_size as usize
        );
        (0..bit_string.len())
            .step_by(4)
            .map(|start| bit_string[start..(start + 4).min(bit_string.len())].to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn update_window(window: &MainWindow, state: &AppState) {
    window.set_mode(state.mode_name.clone().into());
    window.set_display(state.display.clone().into());
    window.set_expression(state.expression.clone().into());
    window.set_status(state.status.clone().into());
    window.set_show_history(state.show_history);
    window.set_show_memory(state.show_memory);
    window.set_expanded_nav(state.expanded_nav);
    window.set_history_text(state.history_text().into());
    window.set_memory_text(state.memory_text().into());
    window.set_programmer_bits(state.programmer_bits().into());
    window.set_programmer_radix(state.radix_name().into());
    window.set_programmer_word_size(format!("{}-bit", state.word_size).into());
    window.set_degrees(ANGLES_IN_DEGREES.load(Ordering::Acquire));
    window.set_converter_category(state.converter_category.name().into());
    let units = state.converter_category.units();
    window.set_converter_units(ModelRc::new(VecModel::from(
        units
            .iter()
            .map(|unit| SharedString::from(unit.name))
            .collect::<Vec<_>>(),
    )));
    window.set_converter_from(units[state.converter_from].name.into());
    window.set_converter_to(units[state.converter_to].name.into());
    window.set_converter_from_index(state.converter_from as i32);
    window.set_converter_to_index(state.converter_to as i32);
    window.set_converter_result(state.converter_result.clone().into());
}

fn main() -> Result<(), slint::PlatformError> {
    let window = MainWindow::new()?;
    let state = Rc::new(RefCell::new(AppState::default()));
    update_window(&window, &state.borrow());

    let weak_window = window.as_weak();
    let state_for_action = state.clone();
    window.on_action(move |action: SharedString| {
        let Some(window) = weak_window.upgrade() else {
            return;
        };
        let mut state = state_for_action.borrow_mut();
        state.sync_display(window.get_display().to_string());
        match action.as_str() {
            "converter-from-selected" => {
                state.converter_from = window.get_converter_from_index() as usize;
            }
            "converter-to-selected" => {
                state.converter_to = window.get_converter_to_index() as usize;
            }
            _ => {}
        }
        state.press(action.as_str());
        update_window(&window, &state);
    });

    let weak_window = window.as_weak();
    let state_for_mode = state.clone();
    window.on_select_mode(move |mode: SharedString| {
        let Some(window) = weak_window.upgrade() else {
            return;
        };
        let mut state = state_for_mode.borrow_mut();
        state.sync_display(window.get_display().to_string());
        state.select_mode(mode.as_str());
        update_window(&window, &state);
    });

    window.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculator_buttons_build_and_evaluate_an_expression() {
        let mut state = AppState::default();
        state.press("2");
        state.press("+");
        state.press("3");
        state.press("×");
        state.press("4");
        state.press("=");

        assert_eq!(state.display, "14");
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0].expression, "2 + 3 * 4");

        state.press("6");
        assert_eq!(state.display, "6");
    }

    #[test]
    fn memory_controls_and_unit_conversion_update_the_display() {
        let mut state = AppState::default();
        state.press("4");
        state.press("2");
        state.press("MS");
        state.press("C");
        state.press("MR");
        assert_eq!(state.display, "42");

        state.press("m-ft");
        assert!((state.display.parse::<f64>().unwrap() - 137.79528).abs() < 1e-8);
    }

    #[test]
    fn programmer_mode_respects_selected_word_size() {
        let mut state = AppState::default();
        state.select_mode("Programmer");
        state.press("word8");
        state.press("2");
        state.press("5");
        state.press("5");
        state.press("NOT");
        assert_eq!(state.display, "0");
    }

    #[test]
    fn history_panel_lists_recent_calculations_and_can_be_cleared() {
        let mut state = AppState::default();
        state.press("9");
        state.press("+");
        state.press("1");
        state.press("=");
        assert!(state.history_text().contains("9 + 1"));
        state.press("clear-history");
        assert!(state.history.is_empty());
        assert_eq!(state.history_text(), "No history yet");
    }

    #[test]
    fn converter_updates_across_categories_and_swaps_units() {
        let mut state = AppState::default();
        state.select_mode("Converter");
        state.press("1");
        state.update_conversion();
        assert!(state.expression.contains("3.280839895013123"));

        state.press("converter-swap");
        assert!(state.expression.contains("0.3048"));

        state.press("converter-category-next");
        assert_eq!(state.converter_category, Category::Area);
        assert_eq!(state.converter_from, 0);
        assert_eq!(state.converter_to, 1);
    }

    #[test]
    fn converter_rejects_below_absolute_zero() {
        let mut state = AppState::default();
        state.select_mode("Converter");
        state.press("converter-category-next");
        state.press("converter-category-next");
        state.press("converter-category-next");
        state.press("converter-category-next");
        state.display = "-300".into();
        state.press("converter-from-previous");
        assert!(state.status.contains("absolute zero"));
    }

    #[test]
    fn converter_uses_manually_edited_input() {
        let mut state = AppState::default();
        state.select_mode("Converter");
        state.sync_display("12".into());
        assert!(state.expression.starts_with("12 m ="));
    }

    #[test]
    fn official_converter_categories_select_the_converter_view() {
        let mut state = AppState::default();
        state.select_mode("Weight and Mass");
        assert_eq!(state.mode_name, "Converter");
        assert_eq!(state.converter_category, Category::Mass);
    }

    #[test]
    fn history_and_memory_panels_are_mutually_exclusive() {
        let mut state = AppState::default();
        state.press("history");
        assert!(state.show_history);
        state.press("memory-panel");
        assert!(!state.show_history);
        assert!(state.show_memory);
    }

    #[test]
    fn programmer_bit_display_groups_bits_into_nibbles() {
        let mut state = AppState::default();
        state.select_mode("Programmer");
        state.press("1");
        let bits = state.programmer_bits();
        assert!(bits.ends_with("0001"));
        assert_eq!(bits.split_whitespace().count(), 16);
    }
}
