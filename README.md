IBAN
----
[![Crates.io][crates-badge]][crates-url]
[![Docs][docs-badge]][docs-url]
[![Coverage Status][codecov-badge]][codecov-url]
![Licensed][license-badge]

[crates-badge]: https://img.shields.io/crates/v/iban
[crates-url]: https://crates.io/crates/iban
[docs-badge]: https://img.shields.io/docsrs/iban/latest
[docs-url]: https://docs.rs/iban/latest/iban
[license-badge]: https://img.shields.io/crates/l/iban
[codecov-badge]: https://img.shields.io/codecov/c/gh/JohnPeel/iban?token=YOLN6DIBGC
[codecov-url]: https://codecov.io/gh/JohnPeel/iban

This is a crate for working with International Bank Account Numbers (IBANs).

An IBAN is an international standard for identifying bank accounts. It consists of a country
code, two check digits, and a Basic Bank Account Number (BBAN) that includes a bank identifier, a branch identifier, and a checksum (if applicable).

This crate provides a simple implementation of IBANs. You can construct an IBAN using a
string, and then query it for its country code, and
check digits. You can also obtain spaced or electronic formatting using the `Display`, `Debug`,
`Deref`, or `AsRef` implementations.

This crate also provides a `Bban` struct that represents the BBAN portion of an IBAN.
You can construct a BBAN using the `Iban::bban` function, and then query it for it's
bank identifier, branch identifier, or checksum.

## Usage

Add library as a dependency to Cargo.toml.

```toml
...
[dependencies]
iban = "0.1"
...
```

Construct a `Iban` type by using `str::parse`, `FromStr::from_str`, or `Iban::parse`.

```rust
use iban::{Bban, Iban};
let iban: Iban = "AA110011123Z5678"
    .parse()
    .unwrap_or_else(|err| {
        // This example panics, but you should handle the error cases properly.
        panic!("invalid iban: {err}");
    });

let country_code: &str = iban.country_code();
let bban: Bban = iban.bban();

let bank_identifier: Option<&str> = bban.bank_identifier();
```

## References
* ISO 13616: https://www.iso13616.org/
* IBAN Registry (pdf): https://www.swift.com/node/9606
* IBAN Registry (txt): https://www.swift.com/node/11971
* php-iban: https://github.com/globalcitizen/php-iban
