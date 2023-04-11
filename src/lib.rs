#![cfg_attr(not(feature = "std"), no_std)]

use core::{fmt, ops::Deref, str::FromStr};

use arrayvec::ArrayString;

mod util;
use util::digits;

include!(concat!(env!("OUT_DIR"), "/countries.rs"));

const IBAN_MAX_LENGTH: usize = 34;

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Iban(ArrayString<IBAN_MAX_LENGTH>);

impl fmt::Debug for Iban {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Display for Iban {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut remaining = &self.0[..];
        while !remaining.is_empty() {
            let (chunk, rest) = remaining.split_at(4.min(remaining.len()));
            remaining = rest;

            if remaining.is_empty() {
                write!(f, "{chunk}")?;
            } else {
                write!(f, "{chunk} ")?;
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ParseError {
    CountryCode,
    CheckDigit,
    InvalidCharacter,
    TooLong,
    UnknownCountry,
    InvalidLength,
    InvalidBban,
    WrongChecksum,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::CountryCode => "invalid country code",
            ParseError::CheckDigit => "invalid check digit",
            ParseError::InvalidCharacter => "invalid character",
            ParseError::TooLong => "too long",
            ParseError::UnknownCountry => "unknown country",
            ParseError::InvalidLength => "invalid length",
            ParseError::InvalidBban => "invalid bban",
            ParseError::WrongChecksum => "checksum validation failed",
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
        &self.0
    }
}

impl AsRef<str> for Iban {
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
                .filter(u8::is_ascii_uppercase)
                .ok_or(ParseError::CountryCode)?;
            iban.push(char::from(ch));
        }

        for _ in 0..2 {
            let ch = characters
                .next()
                .filter(u8::is_ascii_digit)
                .ok_or(ParseError::CheckDigit)?;
            iban.push(char::from(ch));
        }

        for ch in characters {
            if !ch.is_ascii_alphanumeric() {
                return Err(ParseError::InvalidCharacter);
            }

            iban.try_push(char::from(ch))
                .map_err(|_| ParseError::TooLong)?;
        }

        let iban = Self(iban);

        let country_code = iban.country_code();
        let Some(&(expected_length, validation)) = COUNTRIES.get(country_code) else {
            return Err(ParseError::UnknownCountry);
        };

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
                _ => unreachable!(),
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
    #[inline]
    #[must_use]
    pub fn country_code(&self) -> &str {
        &self[0..2]
    }

    #[inline]
    #[must_use]
    pub fn check_digits(&self) -> &str {
        &self[2..4]
    }

    #[inline]
    #[must_use]
    pub fn bban(&self) -> &str {
        &self[4..]
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

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
        let iban = Iban::from_str("AD1200012030200359100100").unwrap();
        assert_eq!(iban.to_string().as_str(), "AD12 0001 2030 2003 5910 0100");

        let iban = Iban::from_str("AE070331234567890123456").unwrap();
        assert_eq!(iban.to_string().as_str(), "AE07 0331 2345 6789 0123 456");
    }

    #[test_case("AA110011123Z5678"; "valid AA iban")]
    #[test_case("AD1200012030200359100100"; "valid AD iban")]
    #[test_case("AE070331234567890123456"; "valid AE iban")]
    #[test_case("AL47212110090000000235698741"; "valid AL iban")]
    #[test_case("AO44123412341234123412341"; "valid AO iban")]
    #[test_case("AT611904300234573201"; "valid AT iban")]
    #[test_case("AX2112345600000785"; "valid AX iban")]
    #[test_case("AZ21NABZ00000000137010001944"; "valid AZ iban")]
    #[test_case("BA391290079401028494"; "valid BA iban")]
    #[test_case("BE68539007547034"; "valid BE iban")]
    #[test_case("BF4512341234123412341234123"; "valid BF iban")]
    #[test_case("BG80BNBG96611020345678"; "valid BG iban")]
    #[test_case("BH67BMAG00001299123456"; "valid BH iban")]
    #[test_case("BI33123412341234"; "valid BI iban")]
    #[test_case("BJ83A12312341234123412341234"; "valid BJ iban")]
    #[test_case("BL6820041010050500013M02606"; "valid BL iban")]
    #[test_case("BR9700360305000010009795493P1"; "valid BR iban")]
    #[test_case("BY13NBRB3600900000002Z00AB00"; "valid BY iban")]
    #[test_case("CF4220001000010120069700160"; "valid CF iban")]
    #[test_case("CG3930013020003710721836132"; "valid CG iban")]
    #[test_case("CH9300762011623852957"; "valid CH iban")]
    #[test_case("CI77A12312341234123412341234"; "valid CI iban")]
    #[test_case("CM1512341234123412341234123"; "valid CM iban")]
    #[test_case("CR05015202001026284066"; "valid CR iban")]
    #[test_case("CV05123412341234123412341"; "valid CV iban")]
    #[test_case("CY17002001280000001200527600"; "valid CY iban")]
    #[test_case("CZ6508000000192000145399"; "valid CZ iban")]
    #[test_case("DE89370400440532013000"; "valid DE iban")]
    #[test_case("DJ2110002010010409943020008"; "valid DJ iban")]
    #[test_case("DK5000400440116243"; "valid DK iban")]
    #[test_case("DO28BAGR00000001212453611324"; "valid DO iban")]
    #[test_case("DZ3512341234123412341234"; "valid DZ iban")]
    #[test_case("EE382200221020145685"; "valid EE iban")]
    #[test_case("EG380019000500000000263180002"; "valid EG iban")]
    #[test_case("ES9121000418450200051332"; "valid ES iban")]
    #[test_case("FI2112345600000785"; "valid FI iban")]
    #[test_case("FO2000400440116243"; "valid FO iban")]
    #[test_case("FR1420041010050500013M02606"; "valid FR iban")]
    #[test_case("GA2142001007341520000106963"; "valid GA iban")]
    #[test_case("GB29NWBK60161331926819"; "valid GB iban")]
    #[test_case("GE29NB0000000101904917"; "valid GE iban")]
    #[test_case("GF4120041010050500013M02606"; "valid GF iban")]
    #[test_case("GI75NWBK000000007099453"; "valid GI iban")]
    #[test_case("GL2000400440116243"; "valid GL iban")]
    #[test_case("GP1120041010050500013M02606"; "valid GP iban")]
    #[test_case("GQ7050002001003715228190196"; "valid GQ iban")]
    #[test_case("GR1601101250000000012300695"; "valid GR iban")]
    #[test_case("GT82TRAJ01020000001210029690"; "valid GT iban")]
    #[test_case("GW04GW1430010181800637601"; "valid GW iban")]
    #[test_case("HN54PISA00000000000000123124"; "valid HN iban")]
    #[test_case("HR1210010051863000160"; "valid HR iban")]
    #[test_case("HU42117730161111101800000000"; "valid HU iban")]
    #[test_case("IE29AIBK93115212345678"; "valid IE iban")]
    #[test_case("IL620108000000099999999"; "valid IL iban")]
    #[test_case("IQ98NBIQ850123456789012"; "valid IQ iban")]
    #[test_case("IR081234123412341234123412"; "valid IR iban")]
    #[test_case("IS140159260076545510730339"; "valid IS iban")]
    #[test_case("IT60X0542811101000000123456"; "valid IT iban")]
    #[test_case("JO94CBJO0010000000000131000302"; "valid JO iban")]
    #[test_case("KM4600005000010010904400137"; "valid KM iban")]
    #[test_case("KW81CBKU0000000000001234560101"; "valid KW iban")]
    #[test_case("KZ86125KZT5004100100"; "valid KZ iban")]
    #[test_case("LB62099900000001001901229114"; "valid LB iban")]
    #[test_case("LC55HEMM000100010012001200023015"; "valid LC iban")]
    #[test_case("LI21088100002324013AA"; "valid LI iban")]
    #[test_case("LT121000011101001000"; "valid LT iban")]
    #[test_case("LU280019400644750000"; "valid LU iban")]
    #[test_case("LV80BANK0000435195001"; "valid LV iban")]
    #[test_case("MA64011519000001205000534921"; "valid MA iban")]
    #[test_case("MC5811222000010123456789030"; "valid MC iban")]
    #[test_case("MD24AG000225100013104168"; "valid MD iban")]
    #[test_case("ME25505000012345678951"; "valid ME iban")]
    #[test_case("MF8420041010050500013M02606"; "valid MF iban")]
    #[test_case("MG4012341234123412341234123"; "valid MG iban")]
    #[test_case("MK07250120000058984"; "valid MK iban")]
    #[test_case("ML75A12312341234123412341234"; "valid ML iban")]
    #[test_case("MQ5120041010050500013M02606"; "valid MQ iban")]
    #[test_case("MR1300020001010000123456753"; "valid MR iban")]
    #[test_case("MT84MALT011000012345MTLCAST001S"; "valid MT iban")]
    #[test_case("MU17BOMM0101101030300200000MUR"; "valid MU iban")]
    #[test_case("MZ97123412341234123412341"; "valid MZ iban")]
    #[test_case("NC8420041010050500013M02606"; "valid NC iban")]
    #[test_case("NE58NE0380100100130305000268"; "valid NE iban")]
    #[test_case("NI92BAMC000000000000000003123123"; "valid NI iban")]
    #[test_case("NL91ABNA0417164300"; "valid NL iban")]
    #[test_case("NO9386011117947"; "valid NO iban")]
    #[test_case("PF5720041010050500013M02606"; "valid PF iban")]
    #[test_case("PK36SCBL0000001123456702"; "valid PK iban")]
    #[test_case("PL61109010140000071219812874"; "valid PL iban")]
    #[test_case("PM3620041010050500013M02606"; "valid PM iban")]
    #[test_case("PS92PALS000000000400123456702"; "valid PS iban")]
    #[test_case("PT50000201231234567890154"; "valid PT iban")]
    #[test_case("QA58DOHB00001234567890ABCDEFG"; "valid QA iban")]
    #[test_case("RE4220041010050500013M02606"; "valid RE iban")]
    #[test_case("RO49AAAA1B31007593840000"; "valid RO iban")]
    #[test_case("RS35260005601001611379"; "valid RS iban")]
    #[test_case("SA0380000000608010167519"; "valid SA iban")]
    #[test_case("SC18SSCB11010000000000001497USD"; "valid SC iban")]
    #[test_case("SE4550000000058398257466"; "valid SE iban")]
    #[test_case("SI56191000000123438"; "valid SI iban")]
    #[test_case("SK3112000000198742637541"; "valid SK iban")]
    #[test_case("SM86U0322509800000000270100"; "valid SM iban")]
    #[test_case("SN15A12312341234123412341234"; "valid SN iban")]
    #[test_case("ST68000100010051845310112"; "valid ST iban")]
    #[test_case("SV62CENR00000000000000700025"; "valid SV iban")]
    #[test_case("TD8960003000203710253860174"; "valid TD iban")]
    #[test_case("TF2120041010050500013M02606"; "valid TF iban")]
    #[test_case("TG53TG0090604310346500400070"; "valid TG iban")]
    #[test_case("TL380080012345678910157"; "valid TL iban")]
    #[test_case("TN5910006035183598478831"; "valid TN iban")]
    #[test_case("TR330006100519786457841326"; "valid TR iban")]
    #[test_case("UA213996220000026007233566001"; "valid UA iban")]
    #[test_case("VG96VPVG0000012345678901"; "valid VG iban")]
    #[test_case("WF9120041010050500013M02606"; "valid WF iban")]
    #[test_case("XK051212012345678906"; "valid XK iban")]
    #[test_case("YT3120041010050500013M02606"; "valid YT iban")]
    fn is_valid(original: &str) {
        let iban = Iban::from_str(original).expect("iban should be valid");

        assert_eq!(iban.country_code(), &original[..2]);
        assert_eq!(iban.check_digits(), &original[2..4]);
        assert_eq!(iban.bban(), &original[4..]);

        assert_eq!(iban.as_ref(), original);
        assert_eq!(&*iban, original);
        assert_eq!(format!("{:?}", iban), format!("{:?}", original));
    }

    #[test_case("aT4120041010050500013M02606", ParseError::CountryCode; "country code")]
    #[test_case("YTa120041010050500013M02606", ParseError::CheckDigit; "check digit")]
    #[test_case("YT412*041010050500013M02606", ParseError::InvalidCharacter; "invalid character")]
    #[test_case("SC18SSCB11010000000000001497USDABCD", ParseError::TooLong; "too long")]
    #[test_case("ZZ18SSCB11010000000000001497USD", ParseError::UnknownCountry; "unknown country")]
    #[test_case("AA110011123Z567891238", ParseError::InvalidLength; "invalid length")]
    #[test_case("YT4120041010050500013M02606", ParseError::WrongChecksum; "wrong checksum")]
    #[test_case("YT3120041010050500013M0260a", ParseError::InvalidBban; "invalid bban")]
    fn assert_error(iban: &str, expected_err: ParseError) {
        assert_eq!(Iban::from_str(iban), Err(expected_err));
        assert!(!expected_err.to_string().is_empty());
    }
}
