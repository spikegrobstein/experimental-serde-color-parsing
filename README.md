# experimental serde color parsing

I'm just keeping this repo around for posterity.

It's some experimental code that I don't want to lose where I'm parsing out values from JSON that may either
be:

 * a string `rainbow`
 * a string with a hex color code like `#ff0000`
 * a string with a short hex color code like `#f00`
 * an array of hex color code strings like `[ "#ff0000", "#000", "#fdfdfd" ]`

This uses an enum as the type of this value with an underlying `Color` type that breaks up the color
components into `u8` values to make it easier to work with.

This will also serialize the same data back to json. Includes tests.

## Acknowledgements

This code is a mix of source from the serde docs themselves + several stack overflow + serde github issue
postings.

The biggest help came from:

 * https://github.com/serde-rs/serde/issues/1131
 * https://serde.rs/string-or-struct.html

