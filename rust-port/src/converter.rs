#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Length,
    Area,
    Volume,
    Mass,
    Temperature,
    Time,
    Speed,
    Energy,
    Pressure,
    Data,
    Power,
    Angle,
}

#[derive(Clone, Copy, Debug)]
pub struct Unit {
    pub name: &'static str,
    pub symbol: &'static str,
    factor: f64,
    offset: f64,
}

const LENGTH: &[Unit] = &[
    unit("Kilometer", "km", 1_000.0),
    unit("Meter", "m", 1.0),
    unit("Centimeter", "cm", 0.01),
    unit("Millimeter", "mm", 0.001),
    unit("Mile", "mi", 1_609.344),
    unit("Yard", "yd", 0.9144),
    unit("Foot", "ft", 0.3048),
    unit("Inch", "in", 0.0254),
];

const AREA: &[Unit] = &[
    unit("Square kilometer", "km²", 1_000_000.0),
    unit("Square meter", "m²", 1.0),
    unit("Square centimeter", "cm²", 0.0001),
    unit("Hectare", "ha", 10_000.0),
    unit("Acre", "ac", 4_046.8564224),
    unit("Square foot", "ft²", 0.09290304),
];

const VOLUME: &[Unit] = &[
    unit("Cubic meter", "m³", 1.0),
    unit("Liter", "L", 0.001),
    unit("Milliliter", "mL", 0.000001),
    unit("US gallon", "gal", 0.003785411784),
    unit("US quart", "qt", 0.000946352946),
    unit("US pint", "pt", 0.000473176473),
    unit("US cup", "cup", 0.0002365882365),
    unit("US fluid ounce", "fl oz", 0.0000295735295625),
];

const MASS: &[Unit] = &[
    unit("Metric ton", "t", 1_000.0),
    unit("Kilogram", "kg", 1.0),
    unit("Gram", "g", 0.001),
    unit("Milligram", "mg", 0.000001),
    unit("Pound", "lb", 0.45359237),
    unit("Ounce", "oz", 0.028349523125),
];

const TEMPERATURE: &[Unit] = &[
    unit_with_offset("Celsius", "°C", 1.0, 273.15),
    unit_with_offset("Fahrenheit", "°F", 5.0 / 9.0, 255.3722222222222),
    unit_with_offset("Kelvin", "K", 1.0, 0.0),
];

const TIME: &[Unit] = &[
    unit("Second", "s", 1.0),
    unit("Minute", "min", 60.0),
    unit("Hour", "h", 3_600.0),
    unit("Day", "d", 86_400.0),
    unit("Week", "wk", 604_800.0),
    unit("Year (365.25 days)", "yr", 31_557_600.0),
];

const SPEED: &[Unit] = &[
    unit("Meter per second", "m/s", 1.0),
    unit("Kilometer per hour", "km/h", 1.0 / 3.6),
    unit("Mile per hour", "mph", 0.44704),
    unit("Foot per second", "ft/s", 0.3048),
    unit("Knot", "kn", 0.5144444444444445),
];

const ENERGY: &[Unit] = &[
    unit("Joule", "J", 1.0),
    unit("Kilojoule", "kJ", 1_000.0),
    unit("Calorie", "cal", 4.184),
    unit("Kilocalorie", "kcal", 4_184.0),
    unit("Watt-hour", "Wh", 3_600.0),
    unit("Kilowatt-hour", "kWh", 3_600_000.0),
    unit("British thermal unit", "BTU", 1_055.05585262),
];

const PRESSURE: &[Unit] = &[
    unit("Pascal", "Pa", 1.0),
    unit("Kilopascal", "kPa", 1_000.0),
    unit("Bar", "bar", 100_000.0),
    unit("Standard atmosphere", "atm", 101_325.0),
    unit("Pound per square inch", "psi", 6_894.757293168),
];

const DATA: &[Unit] = &[
    unit("Bit", "bit", 0.125),
    unit("Byte", "B", 1.0),
    unit("Kilobyte", "KB", 1_000.0),
    unit("Megabyte", "MB", 1_000_000.0),
    unit("Gigabyte", "GB", 1_000_000_000.0),
    unit("Kibibyte", "KiB", 1_024.0),
    unit("Mebibyte", "MiB", 1_048_576.0),
    unit("Gibibyte", "GiB", 1_073_741_824.0),
];

const POWER: &[Unit] = &[
    unit("Watt", "W", 1.0),
    unit("Kilowatt", "kW", 1_000.0),
    unit("Megawatt", "MW", 1_000_000.0),
    unit("Horsepower", "hp", 745.6998715822702),
];

const ANGLE: &[Unit] = &[
    unit("Radian", "rad", 1.0),
    unit("Degree", "°", std::f64::consts::PI / 180.0),
    unit("Gradian", "gon", std::f64::consts::PI / 200.0),
    unit("Turn", "turn", std::f64::consts::TAU),
];

const CATEGORIES: &[Category] = &[
    Category::Length,
    Category::Area,
    Category::Volume,
    Category::Mass,
    Category::Temperature,
    Category::Time,
    Category::Speed,
    Category::Energy,
    Category::Pressure,
    Category::Data,
    Category::Power,
    Category::Angle,
];

const fn unit(name: &'static str, symbol: &'static str, factor: f64) -> Unit {
    Unit {
        name,
        symbol,
        factor,
        offset: 0.0,
    }
}

const fn unit_with_offset(
    name: &'static str,
    symbol: &'static str,
    factor: f64,
    offset: f64,
) -> Unit {
    Unit {
        name,
        symbol,
        factor,
        offset,
    }
}

impl Category {
    pub fn all() -> &'static [Category] {
        CATEGORIES
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Length => "Length",
            Self::Area => "Area",
            Self::Volume => "Volume",
            Self::Mass => "Mass",
            Self::Temperature => "Temperature",
            Self::Time => "Time",
            Self::Speed => "Speed",
            Self::Energy => "Energy",
            Self::Pressure => "Pressure",
            Self::Data => "Data",
            Self::Power => "Power",
            Self::Angle => "Angle",
        }
    }

    pub fn from_nav_name(name: &str) -> Option<Self> {
        let normalized = match name {
            "Weight and Mass" => "Mass",
            other => other,
        };
        CATEGORIES
            .iter()
            .copied()
            .find(|category| category.name() == normalized)
    }

    pub fn units(self) -> &'static [Unit] {
        match self {
            Self::Length => LENGTH,
            Self::Area => AREA,
            Self::Volume => VOLUME,
            Self::Mass => MASS,
            Self::Temperature => TEMPERATURE,
            Self::Time => TIME,
            Self::Speed => SPEED,
            Self::Energy => ENERGY,
            Self::Pressure => PRESSURE,
            Self::Data => DATA,
            Self::Power => POWER,
            Self::Angle => ANGLE,
        }
    }
}

pub fn convert(value: f64, category: Category, from: usize, to: usize) -> Result<f64, String> {
    let units = category.units();
    let from_unit = units
        .get(from)
        .ok_or_else(|| "Select a valid source unit".to_string())?;
    let to_unit = units
        .get(to)
        .ok_or_else(|| "Select a valid destination unit".to_string())?;
    let mut base_value = value * from_unit.factor + from_unit.offset;
    if category == Category::Temperature {
        if base_value < -1e-10 {
            return Err("Temperature cannot be below absolute zero".to_string());
        }
        if base_value < 0.0 {
            base_value = 0.0;
        }
    }
    let result = (base_value - to_unit.offset) / to_unit.factor;
    if result.is_finite() {
        Ok(result)
    } else {
        Err("Conversion result is outside the supported numeric range".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
    }

    #[test]
    fn converts_length_and_temperature() {
        near(
            convert(1.0, Category::Length, 1, 6).unwrap(),
            3.280839895013123,
        );
        near(convert(32.0, Category::Temperature, 1, 0).unwrap(), 0.0);
        near(convert(-40.0, Category::Temperature, 0, 1).unwrap(), -40.0);
    }

    #[test]
    fn supports_metric_imperial_and_binary_data_units() {
        near(
            convert(1.0, Category::Mass, 1, 4).unwrap(),
            2.2046226218487757,
        );
        near(convert(1.0, Category::Data, 5, 1).unwrap(), 1_024.0);
    }

    #[test]
    fn every_unit_pair_round_trips() {
        for category in Category::all() {
            let input = if *category == Category::Temperature {
                20.0
            } else {
                123.456
            };
            for from in 0..category.units().len() {
                for to in 0..category.units().len() {
                    let converted = convert(input, *category, from, to).unwrap();
                    let round_trip = convert(converted, *category, to, from).unwrap();
                    near(round_trip, input);
                }
            }
        }
    }

    #[test]
    fn rejects_invalid_unit_indexes_and_overflow() {
        assert!(convert(1.0, Category::Length, 99, 0).is_err());
        assert!(convert(f64::MAX, Category::Length, 0, 1).is_err());
        assert!(convert(-300.0, Category::Temperature, 0, 2).is_err());
        near(convert(-459.67, Category::Temperature, 1, 2).unwrap(), 0.0);
    }
}
