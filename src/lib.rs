#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

use core::{fmt, ops::Deref, str::FromStr};

use arrayvec::ArrayString;

mod util;
use util::{chunk_str, digits};

include!(concat!(env!("OUT_DIR"), "/countries.rs"));

const IBAN_MAX_LENGTH: usize = 34;

/// Represents an IBAN.
///
/// Use [`FromStr`](std::str::FromStr) to contruct an Iban.
///
/// A valid IBAN satisfies the length defined for that country, has a valid checksum and has
/// a BBAN format as defined in the IBAN registry.
///
/// Spaced formatting can be obtained from the [`Display`](std::fmt::Display) implementation.
///
/// Electronic formatting can be obtained from the [`Debug`](std::fmt::Debug), [`Deref`](std::ops::Deref),
/// or [`AsRef`](std::convert::AsRef) implementations.
///
/// See crate level documentation for more information.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Iban(ArrayString<IBAN_MAX_LENGTH>);

/// Represents a BBAN.
///
/// Use [`Iban::bban`] to obtain this.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Bban(ArrayString<IBAN_MAX_LENGTH>);

impl fmt::Debug for Iban {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl fmt::Debug for Bban {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl fmt::Display for Iban {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = chunk_str(self, 4).peekable();

        while let Some(chunk) = iter.next() {
            write!(f, "{chunk}")?;

            if iter.peek().is_some() {
                write!(f, " ")?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Bban {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = chunk_str(self, 4).peekable();

        while let Some(chunk) = iter.next() {
            write!(f, "{chunk}")?;

            if iter.peek().is_some() {
                write!(f, " ")?;
            }
        }

        Ok(())
    }
}

/// An error indicating the IBAN could not be parsed.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ParseError {
    /// The country code of the IBAN are not uppercase ascii letters.
    CountryCode,
    /// The check digits of the IBAN are not ascii digits.
    CheckDigit,
    /// The IBAN contains a non ascii alphanumeric character.
    InvalidCharacter,
    /// The IBAN is too long to be an IBAN.
    TooLong,
    /// The country of this IBAN is unknown.
    ///
    /// If you're sure that it should be known, please open an issue.
    UnknownCountry,
    /// The IBAN does not match the expected length.
    InvalidLength,
    /// The BBAN does not match the expected format.
    InvalidBban,
    /// Calculating the checksum of the IBAN gave and invalid result.
    WrongChecksum,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CountryCode => "invalid country code",
            Self::CheckDigit => "invalid check digit",
            Self::InvalidCharacter => "invalid character",
            Self::TooLong => "too long",
            Self::UnknownCountry => "unknown country",
            Self::InvalidLength => "invalid length",
            Self::InvalidBban => "invalid bban",
            Self::WrongChecksum => "checksum validation failed",
        }
        .fmt(f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}

impl Deref for Iban {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl Deref for Bban {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0[4..]
    }
}

impl AsRef<str> for Iban {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl AsRef<str> for Bban {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl FromStr for Iban {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut iban = ArrayString::<IBAN_MAX_LENGTH>::new();
        let mut characters = value
            .as_bytes()
            .iter()
            .copied()
            .filter(|byte| !byte.is_ascii_whitespace());

        for _ in 0..2 {
            let ch = characters
                .next()
                // SAFETY: This condition is tied with an unsafe block below.
                .filter(u8::is_ascii_uppercase)
                .ok_or(ParseError::CountryCode)?;
            iban.push(char::from(ch));
        }

        for _ in 0..2 {
            let ch = characters
                .next()
                // SAFETY: This condition is tied with an unsafe block below.
                .filter(u8::is_ascii_digit)
                .ok_or(ParseError::CheckDigit)?;
            iban.push(char::from(ch));
        }

        for ch in characters {
            // SAFETY: This condition is tied with an unsafe block below.
            if !ch.is_ascii_alphanumeric() {
                return Err(ParseError::InvalidCharacter);
            }

            iban.try_push(char::from(ch))
                .map_err(|_| ParseError::TooLong)?;
        }

        let iban = Self(iban);

        let country_code = iban.country_code();
        let &(expected_length, validation, ..) = COUNTRIES
            .get(country_code)
            .ok_or(ParseError::UnknownCountry)?;

        if expected_length != iban.len() {
            return Err(ParseError::InvalidLength);
        }

        let valid = validation
            .iter()
            .flat_map(|(count, character_type)| (0..*count).map(move |_| character_type))
            .zip(iban.as_bytes())
            .all(|(character_type, character)| match character_type {
                'n' => character.is_ascii_digit(),
                'a' => character.is_ascii_uppercase(),
                'i' => character.is_ascii_uppercase() || character.is_ascii_digit(),
                'c' => character.is_ascii_alphanumeric(),
                expected => char::from(*character) == *expected,
            });

        if !valid {
            return Err(ParseError::InvalidBban);
        }

        let checksum = iban
            .bban()
            .bytes()
            .chain(iban.country_code().bytes())
            .chain(iban.check_digits().bytes())
            .flat_map(|character| match character {
                b'0'..=b'9' => digits(character - b'0'),
                b'a'..=b'z' => digits(character - b'a' + 10),
                b'A'..=b'Z' => digits(character - b'A' + 10),
                // SAFETY: Any characters that are not alphanumeric would have errored before the checksum validation.
                // * Country code must be uppercased ascii letters.
                // * Check digits must be ascii numbers.
                // * BBAN must be ascii alphanumeric.
                _ => unsafe { core::hint::unreachable_unchecked() },
            })
            .fold(0u32, |checksum, item| {
                let checksum = checksum * 10 + u32::from(item);
                if checksum > 9_999_999 {
                    checksum % 97
                } else {
                    checksum
                }
            })
            % 97;

        if checksum != 1 {
            return Err(ParseError::WrongChecksum);
        }

        Ok(iban)
    }
}

impl Iban {
    /// Get the country code of the IBAN.
    #[inline]
    #[must_use]
    pub fn country_code(&self) -> &str {
        &self[0..2]
    }

    /// Get the check digits of the IBAN.
    #[inline]
    #[must_use]
    pub fn check_digits(&self) -> &str {
        &self[2..4]
    }

    /// Get the BBAN of the IBAN.
    #[inline]
    #[must_use]
    pub const fn bban(&self) -> Bban {
        Bban(self.0)
    }

    /// Parse a string as an Iban.
    #[inline]
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        FromStr::from_str(s)
    }
}

impl Bban {
    #[inline]
    #[must_use]
    fn country_code(&self) -> &str {
        &self.0[0..2]
    }

    /// Get the bank identifier of the BBAN (if it has one).
    #[inline]
    #[must_use]
    pub fn bank_identifier(&self) -> Option<&str> {
        let (_expected_length, _validation, bank_offset, _branch_offset, _checksum_offset) =
            COUNTRIES.get(self.country_code())?;
        bank_offset
            .as_ref()
            .copied()
            .map(|(start, end)| &self[start..end])
    }

    /// Get the branch identifier of the BBAN (if it has one).
    #[inline]
    #[must_use]
    pub fn branch_identifier(&self) -> Option<&str> {
        let (_expected_length, _validation, _bank_offset, branch_offset, _checksum_offset) =
            COUNTRIES.get(self.country_code())?;
        branch_offset
            .as_ref()
            .copied()
            .map(|(start, end)| &self[start..end])
    }

    /// Get the checksum of the BBAN (if it has one).
    #[inline]
    #[must_use]
    pub fn checksum(&self) -> Option<&str> {
        let (_expected_length, _validation, _bank_offset, _branch_offset, checksum_offset) =
            COUNTRIES.get(self.country_code())?;
        checksum_offset
            .as_ref()
            .copied()
            .map(|(start, end)| &self[start..end])
    }
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use crate::{digits, Iban, ParseError};

    #[test]
    fn simple_digits() {
        for i in 0..9 {
            assert_eq!(digits(i).collect::<Vec<_>>(), vec![i]);
        }

        for i in 10..36 {
            assert_eq!(digits(i).collect::<Vec<_>>(), vec![i / 10, i % 10]);
        }
    }

    #[test]
    fn iban_display_impl() {
        let iban = Iban::parse("AD1200012030200359100100").unwrap();
        assert_eq!(iban.to_string().as_str(), "AD12 0001 2030 2003 5910 0100");

        let iban = Iban::parse("AE070331234567890123456").unwrap();
        assert_eq!(iban.to_string().as_str(), "AE07 0331 2345 6789 0123 456");
    }

    #[test]
    fn bban_display_impl() {
        let iban = Iban::parse("AD1200012030200359100100").unwrap();
        let bban = iban.bban();
        assert_eq!(bban.to_string().as_str(), "0001 2030 2003 5910 0100");

        let iban = Iban::parse("AE070331234567890123456").unwrap();
        let bban = iban.bban();
        assert_eq!(bban.to_string().as_str(), "0331 2345 6789 0123 456");
    }

    #[test_case("AA110011123Z5678"; "AA")]
    #[test_case("AD1200012030200359100100"; "AD")]
    #[test_case("AE070331234567890123456"; "AE")]
    #[test_case("AL47212110090000000235698741"; "AL")]
    #[test_case("AO44123412341234123412341"; "AO")]
    #[test_case("AT611904300234573201"; "AT")]
    #[test_case("AX2112345600000785"; "AX")]
    #[test_case("AZ21NABZ00000000137010001944"; "AZ")]
    #[test_case("BA391290079401028494"; "BA")]
    #[test_case("BE68539007547034"; "BE")]
    #[test_case("BF4512341234123412341234123"; "BF")]
    #[test_case("BG80BNBG96611020345678"; "BG")]
    #[test_case("BH67BMAG00001299123456"; "BH")]
    #[test_case("BI33123412341234"; "BI")]
    #[test_case("BJ83A12312341234123412341234"; "BJ")]
    #[test_case("BL6820041010050500013M02606"; "BL")]
    #[test_case("BR9700360305000010009795493P1"; "BR")]
    #[test_case("BY13NBRB3600900000002Z00AB00"; "BY")]
    #[test_case("CF4220001000010120069700160"; "CF")]
    #[test_case("CG3930013020003710721836132"; "CG")]
    #[test_case("CH9300762011623852957"; "CH")]
    #[test_case("CI77A12312341234123412341234"; "CI")]
    #[test_case("CM1512341234123412341234123"; "CM")]
    #[test_case("CR05015202001026284066"; "CR")]
    #[test_case("CV05123412341234123412341"; "CV")]
    #[test_case("CY17002001280000001200527600"; "CY")]
    #[test_case("CZ6508000000192000145399"; "CZ")]
    #[test_case("DE89370400440532013000"; "DE")]
    #[test_case("DJ2110002010010409943020008"; "DJ")]
    #[test_case("DK5000400440116243"; "DK")]
    #[test_case("DO28BAGR00000001212453611324"; "DO_")]
    #[test_case("DZ3512341234123412341234"; "DZ")]
    #[test_case("EE382200221020145685"; "EE")]
    #[test_case("EG380019000500000000263180002"; "EG")]
    #[test_case("ES9121000418450200051332"; "ES")]
    #[test_case("FI2112345600000785"; "FI")]
    #[test_case("FO2000400440116243"; "FO")]
    #[test_case("FR1420041010050500013M02606"; "FR")]
    #[test_case("GA2142001007341520000106963"; "GA")]
    #[test_case("GB29NWBK60161331926819"; "GB")]
    #[test_case("GE29NB0000000101904917"; "GE")]
    #[test_case("GF4120041010050500013M02606"; "GF")]
    #[test_case("GI75NWBK000000007099453"; "GI")]
    #[test_case("GL2000400440116243"; "GL")]
    #[test_case("GP1120041010050500013M02606"; "GP")]
    #[test_case("GQ7050002001003715228190196"; "GQ")]
    #[test_case("GR1601101250000000012300695"; "GR")]
    #[test_case("GT82TRAJ01020000001210029690"; "GT")]
    #[test_case("GW04GW1430010181800637601"; "GW")]
    #[test_case("HN54PISA00000000000000123124"; "HN")]
    #[test_case("HR1210010051863000160"; "HR")]
    #[test_case("HU42117730161111101800000000"; "HU")]
    #[test_case("IE29AIBK93115212345678"; "IE")]
    #[test_case("IL620108000000099999999"; "IL")]
    #[test_case("IQ98NBIQ850123456789012"; "IQ")]
    #[test_case("IR081234123412341234123412"; "IR")]
    #[test_case("IS140159260076545510730339"; "IS")]
    #[test_case("IT60X0542811101000000123456"; "IT")]
    #[test_case("JO94CBJO0010000000000131000302"; "JO")]
    #[test_case("KM4600005000010010904400137"; "KM")]
    #[test_case("KW81CBKU0000000000001234560101"; "KW")]
    #[test_case("KZ86125KZT5004100100"; "KZ")]
    #[test_case("LB62099900000001001901229114"; "LB")]
    #[test_case("LC55HEMM000100010012001200023015"; "LC")]
    #[test_case("LI21088100002324013AA"; "LI")]
    #[test_case("LT121000011101001000"; "LT")]
    #[test_case("LU280019400644750000"; "LU")]
    #[test_case("LV80BANK0000435195001"; "LV")]
    #[test_case("MA64011519000001205000534921"; "MA")]
    #[test_case("MC5811222000010123456789030"; "MC")]
    #[test_case("MD24AG000225100013104168"; "MD")]
    #[test_case("ME25505000012345678951"; "ME")]
    #[test_case("MF8420041010050500013M02606"; "MF")]
    #[test_case("MG4012341234123412341234123"; "MG")]
    #[test_case("MK07250120000058984"; "MK")]
    #[test_case("ML75A12312341234123412341234"; "ML")]
    #[test_case("MQ5120041010050500013M02606"; "MQ")]
    #[test_case("MR1300020001010000123456753"; "MR")]
    #[test_case("MT84MALT011000012345MTLCAST001S"; "MT")]
    #[test_case("MU17BOMM0101101030300200000MUR"; "MU")]
    #[test_case("MZ97123412341234123412341"; "MZ")]
    #[test_case("NC8420041010050500013M02606"; "NC")]
    #[test_case("NE58NE0380100100130305000268"; "NE")]
    #[test_case("NI92BAMC000000000000000003123123"; "NI")]
    #[test_case("NL91ABNA0417164300"; "NL")]
    #[test_case("NO9386011117947"; "NO")]
    #[test_case("PF5720041010050500013M02606"; "PF")]
    #[test_case("PK36SCBL0000001123456702"; "PK")]
    #[test_case("PL61109010140000071219812874"; "PL")]
    #[test_case("PM3620041010050500013M02606"; "PM")]
    #[test_case("PS92PALS000000000400123456702"; "PS")]
    #[test_case("PT50000201231234567890154"; "PT")]
    #[test_case("QA58DOHB00001234567890ABCDEFG"; "QA")]
    #[test_case("RE4220041010050500013M02606"; "RE")]
    #[test_case("RO49AAAA1B31007593840000"; "RO")]
    #[test_case("RS35260005601001611379"; "RS")]
    #[test_case("SA0380000000608010167519"; "SA")]
    #[test_case("SC18SSCB11010000000000001497USD"; "SC")]
    #[test_case("SE4550000000058398257466"; "SE")]
    #[test_case("SI56191000000123438"; "SI")]
    #[test_case("SK3112000000198742637541"; "SK")]
    #[test_case("SM86U0322509800000000270100"; "SM")]
    #[test_case("SN15A12312341234123412341234"; "SN")]
    #[test_case("ST68000100010051845310112"; "ST")]
    #[test_case("SV62CENR00000000000000700025"; "SV")]
    #[test_case("TD8960003000203710253860174"; "TD")]
    #[test_case("TF2120041010050500013M02606"; "TF")]
    #[test_case("TG53TG0090604310346500400070"; "TG")]
    #[test_case("TL380080012345678910157"; "TL")]
    #[test_case("TN5910006035183598478831"; "TN")]
    #[test_case("TR330006100519786457841326"; "TR")]
    #[test_case("UA213996220000026007233566001"; "UA")]
    #[test_case("VG96VPVG0000012345678901"; "VG")]
    #[test_case("WF9120041010050500013M02606"; "WF")]
    #[test_case("XK051212012345678906"; "XK")]
    #[test_case("YT3120041010050500013M02606"; "YT")]
    fn iban(original: &str) {
        let iban = Iban::parse(original).expect("iban should be valid");

        assert_eq!(iban.country_code(), &original[..2]);
        assert_eq!(iban.check_digits(), &original[2..4]);
        assert_eq!(&*iban.bban(), &original[4..]);

        assert_eq!(iban.as_ref(), original);
        assert_eq!(&*iban, original);
        assert_eq!(format!("{:?}", iban), format!("{:?}", original));

        let iban2 = iban;
        assert_eq!(iban, iban2);
    }

    #[test_case("aT4120041010050500013M02606", ParseError::CountryCode; "country code")]
    #[test_case("YTa120041010050500013M02606", ParseError::CheckDigit; "check digit")]
    #[test_case("YT412*041010050500013M02606", ParseError::InvalidCharacter; "invalid character")]
    #[test_case("SC18SSCB11010000000000001497USDABCD", ParseError::TooLong; "too long")]
    #[test_case("ZZ18SSCB11010000000000001497USD", ParseError::UnknownCountry; "unknown country")]
    #[test_case("AA110011123Z567891238", ParseError::InvalidLength; "invalid length")]
    #[test_case("YT4120041010050500013M02606", ParseError::WrongChecksum; "wrong checksum")]
    #[test_case("YT3120041010050500013M0260a", ParseError::InvalidBban; "invalid bban")]
    fn parse_error(iban: &str, expected_err: ParseError) {
        assert_eq!(Iban::parse(iban), Err(expected_err));
        assert!(!expected_err.to_string().is_empty());
    }

    // This is only valid because BL's IBAN format allows lowercase in that position.
    // `BL2!n5!n5!n11!c2!n`, specifically the `11!c`.
    #[test_case("BL6820041010050500013M02606"; "uppercase BL")]
    #[test_case("BL6820041010050500013m02606"; "lowercase BL")]
    fn case_sensitivity(iban: &str) {
        assert!(Iban::parse(iban).is_ok());
    }

    #[test_case("BL6820041010050500013M02606", Some("20041"), Some("01005"), Some("06"); "BL")]
    #[test_case("AA110011123Z5678", Some("0011"), None, None; "AA")]
    #[test_case("BE68539007547034", Some("539"), None, Some("34"); "BE")]
    #[test_case("IQ98NBIQ850123456789012", Some("NBIQ"), Some("850"), None; "IQ")]
    fn bban(original: &str, bank: Option<&str>, branch: Option<&str>, checksum: Option<&str>) {
        let iban = Iban::parse(original).expect("iban is valid");
        let bban = iban.bban();
        assert_eq!(&iban[4..], bban.as_ref());

        assert_eq!(&*bban, &original[4..]);
        assert_eq!(format!("{:?}", bban), format!("{:?}", &original[4..]));

        assert_eq!(bban.bank_identifier(), bank);
        assert_eq!(bban.branch_identifier(), branch);
        assert_eq!(bban.checksum(), checksum);

        let bban2 = bban;
        assert_eq!(bban, bban2);
    }
}
