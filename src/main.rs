use serde::{Deserialize, Deserializer};
use serde_json::Result;
use serde::de::{self, Visitor, SeqAccess};

use std::str::FromStr;
use std::marker::PhantomData;
use std::fmt;

use thiserror::Error;

#[derive(Deserialize, Debug)]
struct MyData {
    pub color: Fill,
}

#[derive(Error, Debug, PartialEq)]
enum ColorParser {
    #[error("Missing leading '#' descriptor")]
    MissingPrefix,

    #[error("Invalid length")]
    InvalidLength(usize),
}

#[derive(Deserialize, Debug, PartialEq)]
struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl FromStr for Color {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let len = s.len();

        if s.chars().nth(0) != Some('#') {
            return Err(ColorParser::MissingPrefix.into())
        }

        let s = &s[1..];
        eprintln!("parsing: {}", s);

        let (red, green, blue) =
            match len {
                4 => {
                    // 17 * c
                    let red = 17 * u8::from_str_radix(&s[0..1], 16)?;
                    let green = 17 * u8::from_str_radix(&s[1..2], 16)?;
                    let blue = 17 * u8::from_str_radix(&s[2..3], 16)?;

                    (red, green, blue)
                },
                7 => {
                    // parse the double-digit hex value
                    let red = u8::from_str_radix(&s[0..=1], 16)?;
                    let green = u8::from_str_radix(&s[2..=3], 16)?;
                    let blue = u8::from_str_radix(&s[4..=5], 16)?;

                    (red, green, blue)
                },
                len => {
                    return Err(ColorParser::InvalidLength(len).into())
                }
            };

        Ok(Color {
            red,
            green,
            blue,
        })
    }
}

#[derive(Debug, PartialEq)]
// #[serde(untagged)]
enum Fill {
    Rainbow,
    Color(Color),
    Gradient(Vec<Color>),
}

impl FromStr for Fill {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        eprintln!("calling fromstr: {}", s);
        let res = match s {
            "rainbow" => Fill::Rainbow,
            s => Fill::Color(Color::from_str(s)?),
        };

        Ok(res)
    }
}

impl<'de> Deserialize<'de> for Fill {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // This is a Visitor that forwards string types to T's `FromStr` impl and
        // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
        // keep the compiler from complaining about T being an unused generic type
        // parameter. We need T in order to know the Value type for the Visitor
        // impl.
        struct StringOrVec<Fill>(PhantomData<fn() -> Fill>);

        impl<'de> Visitor<'de> for StringOrVec<Fill>
        {
            type Value = Fill;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string or array")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Fill, E>
            where
                E: de::Error,
            {
                Ok(FromStr::from_str(value).unwrap())
            }

            fn visit_seq<S>(self, mut seq: S) -> std::result::Result<Fill, S::Error>
            where
                S: SeqAccess<'de>,
            {
                // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
                // into a `Deserializer`, allowing it to be used as the input to T's
                // `Deserialize` implementation. T then deserializes itself using
                // the entries from the map visitor.

                let mut colors: Vec<Color> = vec![];

                while let Some(c) = seq.next_element()? {
                    colors.push(FromStr::from_str(c).unwrap());
                }

                Ok(Fill::Gradient(colors))

                // Deserialize::deserialize(de::value::SeqAccessDeserializer::new(seq))
            }
        }

        deserializer.deserialize_any(StringOrVec(PhantomData))
    }
}

fn main() -> Result<()> {
    let data = r##"
        { "color": "#ffdd00" }
    "##;

    // Parse the string of data into serde_json::Value.
    let v: MyData = serde_json::from_str(data)?;

    // Access parts of the data by indexing with square brackets.
    println!("color: {:?}", v.color);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_rainbow() {
        let data = r##"
            { "color": "rainbow" }
        "##;

        let v: MyData = serde_json::from_str(data).unwrap();

        assert_eq!(v.color, Fill::Rainbow);
    }

    #[test]
    fn it_parses_short_color() {
        let data = r##"
            { "color": "#f0f" }
        "##;

        let v: MyData = serde_json::from_str(data).unwrap();

        assert_eq!(v.color, Fill::Color(Color { red: 255, green: 0, blue: 255 }));
    }

    #[test]
    fn it_parses_long_color() {
        let data = r##"
            { "color": "#ff00ff" }
        "##;

        let v: MyData = serde_json::from_str(data).unwrap();

        assert_eq!(v.color, Fill::Color(Color { red: 255, green: 0, blue: 255 }));
    }

    #[test]
    fn it_parses_a_gradient() {
        let data = r##"
            { "color": [ "#fff", "#00ff00", "#00f" ] }
        "##;

        let v: MyData = serde_json::from_str(data).unwrap();

        assert_eq!(v.color, Fill::Gradient(vec![
            Color { red: 255, green: 255, blue: 255 },
            Color { red: 0, green: 255, blue: 0 },
            Color { red: 0, green: 0, blue: 255 },
        ]));
    }

    #[test]
    #[should_panic]
    fn it_fails_with_random_string() {
        let data = r##"
            { "color": "hello" }
        "##;

        serde_json::from_str::<MyData>(data).unwrap();
    }

    #[test]
    #[should_panic]
    fn it_fails_with_short_string() {
        let data = r##"
            { "color": "#f" }
        "##;

        serde_json::from_str::<MyData>(data).unwrap();
    }

    #[test]
    #[should_panic]
    fn it_fails_with_long_string() {
        let data = r##"
            { "color": "#fffffffffffffff" }
        "##;

        serde_json::from_str::<MyData>(data).unwrap();
    }

    #[test]
    #[should_panic]
    fn it_fails_with_gradient_rainbow() {
        let data = r##"
            { "color": ["rainbow"] }
        "##;

        serde_json::from_str::<MyData>(data).unwrap();
    }

    #[test]
    #[should_panic]
    fn it_fails_with_gradient_random_string() {
        let data = r##"
            { "color": ["hello"] }
        "##;

        serde_json::from_str::<MyData>(data).unwrap();
    }

    #[test]
    #[should_panic]
    fn it_fails_with_gradient_short_string() {
        let data = r##"
            { "color": ["#f"] }
        "##;

        serde_json::from_str::<MyData>(data).unwrap();
    }

    #[test]
    #[should_panic]
    fn it_fails_with_gradient_long_string() {
        let data = r##"
            { "color": ["#fffffffffffffff"] }
        "##;

        serde_json::from_str::<MyData>(data).unwrap();
    }
}
