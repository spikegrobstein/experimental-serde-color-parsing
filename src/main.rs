use serde::{Serialize, Serializer, Deserialize, Deserializer};
use serde_json::Result;
use serde_json::json;
use serde::de::{self, Visitor, SeqAccess};
use serde::ser::SerializeSeq;

use std::str::FromStr;
use std::marker::PhantomData;
use std::fmt;

use thiserror::Error;

#[derive(Deserialize, Serialize, Debug)]
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

#[derive(Debug, PartialEq)]
struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
    }
}


impl FromStr for Color {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let len = s.len();

        if s.chars().nth(0) != Some('#') {
            return Err(ColorParser::MissingPrefix.into())
        }

        let s = &s[1..];

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

impl Serialize for Fill
{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serializer.serialize_str("foo")
        
        match self {
            Fill::Rainbow => serializer.serialize_str("rainbow"),
            Fill::Color(color) => {
                serializer.serialize_str(&format!{"{}", color})
            },
            Fill::Gradient(colors) => {
                let mut s = serializer.serialize_seq(Some(colors.len()))?;
                for c in colors {
                    s.serialize_element(&format!("{}", c))?;
                }

                s.end()
            }
        }
    }
}

fn main() -> Result<()> {
    let data = r##"
        { "color": "rainbow" }
    "##;

    // Parse the string of data into serde_json::Value.
    let v: MyData = serde_json::from_str(data)?;

    // Access parts of the data by indexing with square brackets.
    println!("color: {:?}", v.color);

    let json = json!(v);

    println!("json: {}", json);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod deserialize {
        use super::*;

        #[test]
        fn rainbow() {
            let data = r##"
                { "color": "rainbow" }
            "##;

            let v: MyData = serde_json::from_str(data).unwrap();

            assert_eq!(v.color, Fill::Rainbow);
        }

        #[test]
        fn short_color() {
            let data = r##"
                { "color": "#f0f" }
            "##;

            let v: MyData = serde_json::from_str(data).unwrap();

            assert_eq!(v.color, Fill::Color(Color { red: 255, green: 0, blue: 255 }));
        }

        #[test]
        fn long_color() {
            let data = r##"
                { "color": "#ff00ff" }
            "##;

            let v: MyData = serde_json::from_str(data).unwrap();

            assert_eq!(v.color, Fill::Color(Color { red: 255, green: 0, blue: 255 }));
        }

        #[test]
        fn gradient() {
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
        fn arbitrary_string_fails() {
            let data = r##"
                { "color": "hello" }
            "##;

            serde_json::from_str::<MyData>(data).unwrap();
        }

        #[test]
        #[should_panic]
        fn short_string_fails() {
            let data = r##"
                { "color": "#f" }
            "##;

            serde_json::from_str::<MyData>(data).unwrap();
        }

        #[test]
        #[should_panic]
        fn too_long_of_string_fails() {
            let data = r##"
                { "color": "#fffffffffffffff" }
            "##;

            serde_json::from_str::<MyData>(data).unwrap();
        }

        #[test]
        #[should_panic]
        fn rainbow_in_gradient_fails() {
            let data = r##"
                { "color": ["rainbow"] }
            "##;

            serde_json::from_str::<MyData>(data).unwrap();
        }

        #[test]
        #[should_panic]
        fn arbitrary_string_in_gradient_fails() {
            let data = r##"
                { "color": ["hello"] }
            "##;

            serde_json::from_str::<MyData>(data).unwrap();
        }

        #[test]
        #[should_panic]
        fn short_string_in_gradient_fails() {
            let data = r##"
                { "color": ["#f"] }
            "##;

            serde_json::from_str::<MyData>(data).unwrap();
        }

        #[test]
        #[should_panic]
        fn long_string_in_gradient_fails() {
            let data = r##"
                { "color": ["#fffffffffffffff"] }
            "##;

            serde_json::from_str::<MyData>(data).unwrap();
        }
    }

    mod ser {
        use super::*;

        #[test]
        fn rainbow() {
            let json = json!(MyData { color: Fill::Rainbow });

            assert_eq!(json.to_string(), r##"{"color":"rainbow"}"##)
        }

        #[test]
        fn color() {
            let json = json!(MyData { color: Fill::Color(Color { red: 255, green: 255, blue: 255 })});

            assert_eq!(json.to_string(), r##"{"color":"#ffffff"}"##);

            let json = json!(MyData { color: Fill::Color(Color { red: 15, green: 0, blue: 255 })});

            assert_eq!(json.to_string(), r##"{"color":"#0f00ff"}"##)
        }

        #[test]
        fn gradient() {
            let json = json!(MyData { color: Fill::Gradient(vec![
                Color { red: 255, green: 255, blue: 255 },
                Color { red: 15, green: 0, blue: 255 },
            ])});

            assert_eq!(json.to_string(), r##"{"color":["#ffffff","#0f00ff"]}"##)
        }
    }
}
