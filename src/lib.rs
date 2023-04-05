#![cfg_attr(not(feature = "std"), no_std)]

use core::{fmt, ops::Deref, str::FromStr};

use arrayvec::ArrayString;

mod util;
use util::IteratorExt as _;

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

fn digits(mut value: u8) -> impl Iterator<Item = u8> {
    let hundreds = value / 100;
    value -= hundreds * 100;
    let tens = value / 10;
    value -= tens * 10;
    let ones = value;

    [hundreds, tens, ones]
        .into_iter()
        // Skip leading zeros
        .skip_while(|&b| b == 0)
        // Ensure at least one value (0) is provided by this iterator.
        .ensure_one(0)
}

impl FromStr for Iban {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut iban = ArrayString::<IBAN_MAX_LENGTH>::new();
        let mut characters = value
            .as_bytes()
            .iter()
            .copied()
            .filter(u8::is_ascii_alphanumeric);

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

    use super::{digits, Iban};

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

    #[test]
    fn examples_from_registry() {
        Iban::from_str("AA110011123Z5678").expect("valid iiban");
        Iban::from_str("AD1200012030200359100100").expect("valid iban for AD");
        Iban::from_str("AE070331234567890123456").expect("valid iban for AE");
        Iban::from_str("AL47212110090000000235698741").expect("valid iban for AL");
        Iban::from_str("AO44123412341234123412341").expect("valid iban for AO");
        Iban::from_str("AT611904300234573201").expect("valid iban for AT");
        Iban::from_str("AX2112345600000785").expect("valid iban for AX");
        Iban::from_str("AZ21NABZ00000000137010001944").expect("valid iban for AZ");
        Iban::from_str("BA391290079401028494").expect("valid iban for BA");
        Iban::from_str("BE68539007547034").expect("valid iban for BE");
        Iban::from_str("BF4512341234123412341234123").expect("valid iban for BF");
        Iban::from_str("BG80BNBG96611020345678").expect("valid iban for BG");
        Iban::from_str("BH67BMAG00001299123456").expect("valid iban for BH");
        Iban::from_str("BI33123412341234").expect("valid iban for BI");
        Iban::from_str("BJ83A12312341234123412341234").expect("valid iban for BJ");
        Iban::from_str("BL6820041010050500013M02606").expect("valid iban for BL");
        Iban::from_str("BR9700360305000010009795493P1").expect("valid iban for BR");
        Iban::from_str("BY13NBRB3600900000002Z00AB00").expect("valid iban for BY");
        Iban::from_str("CF4220001000010120069700160").expect("valid iban for CF");
        Iban::from_str("CG3930013020003710721836132").expect("valid iban for CG");
        Iban::from_str("CH9300762011623852957").expect("valid iban for CH");
        Iban::from_str("CI77A12312341234123412341234").expect("valid iban for CI");
        Iban::from_str("CM1512341234123412341234123").expect("valid iban for CM");
        Iban::from_str("CR05015202001026284066").expect("valid iban for CR");
        Iban::from_str("CV05123412341234123412341").expect("valid iban for CV");
        Iban::from_str("CY17002001280000001200527600").expect("valid iban for CY");
        Iban::from_str("CZ6508000000192000145399").expect("valid iban for CZ");
        Iban::from_str("DE89370400440532013000").expect("valid iban for DE");
        Iban::from_str("DJ2110002010010409943020008").expect("valid iban for DJ");
        Iban::from_str("DK5000400440116243").expect("valid iban for DK");
        Iban::from_str("DO28BAGR00000001212453611324").expect("valid iban for DO");
        Iban::from_str("DZ3512341234123412341234").expect("valid iban for DZ");
        Iban::from_str("EE382200221020145685").expect("valid iban for EE");
        Iban::from_str("EG380019000500000000263180002").expect("valid iban for EG");
        Iban::from_str("ES9121000418450200051332").expect("valid iban for ES");
        Iban::from_str("FI2112345600000785").expect("valid iban for FI");
        Iban::from_str("FO2000400440116243").expect("valid iban for FO");
        Iban::from_str("FR1420041010050500013M02606").expect("valid iban for FR");
        Iban::from_str("GA2142001007341520000106963").expect("valid iban for GA");
        Iban::from_str("GB29NWBK60161331926819").expect("valid iban for GB");
        Iban::from_str("GE29NB0000000101904917").expect("valid iban for GE");
        Iban::from_str("GF4120041010050500013M02606").expect("valid iban for GF");
        Iban::from_str("GI75NWBK000000007099453").expect("valid iban for GI");
        Iban::from_str("GL2000400440116243").expect("valid iban for GL");
        Iban::from_str("GP1120041010050500013M02606").expect("valid iban for GP");
        Iban::from_str("GQ7050002001003715228190196").expect("valid iban for GQ");
        Iban::from_str("GR1601101250000000012300695").expect("valid iban for GR");
        Iban::from_str("GT82TRAJ01020000001210029690").expect("valid iban for GT");
        Iban::from_str("GW04GW1430010181800637601").expect("valid iban for GW");
        Iban::from_str("HN54PISA00000000000000123124").expect("valid iban for HN");
        Iban::from_str("HR1210010051863000160").expect("valid iban for HR");
        Iban::from_str("HU42117730161111101800000000").expect("valid iban for HU");
        Iban::from_str("IE29AIBK93115212345678").expect("valid iban for IE");
        Iban::from_str("IL620108000000099999999").expect("valid iban for IL");
        Iban::from_str("IQ98NBIQ850123456789012").expect("valid iban for IQ");
        Iban::from_str("IR081234123412341234123412").expect("valid iban for IR");
        Iban::from_str("IS140159260076545510730339").expect("valid iban for IS");
        Iban::from_str("IT60X0542811101000000123456").expect("valid iban for IT");
        Iban::from_str("JO94CBJO0010000000000131000302").expect("valid iban for JO");
        Iban::from_str("KM4600005000010010904400137").expect("valid iban for KM");
        Iban::from_str("KW81CBKU0000000000001234560101").expect("valid iban for KW");
        Iban::from_str("KZ86125KZT5004100100").expect("valid iban for KZ");
        Iban::from_str("LB62099900000001001901229114").expect("valid iban for LB");
        Iban::from_str("LC55HEMM000100010012001200023015").expect("valid iban for LC");
        Iban::from_str("LI21088100002324013AA").expect("valid iban for LI");
        Iban::from_str("LT121000011101001000").expect("valid iban for LT");
        Iban::from_str("LU280019400644750000").expect("valid iban for LU");
        Iban::from_str("LV80BANK0000435195001").expect("valid iban for LV");
        Iban::from_str("MA64011519000001205000534921").expect("valid iban for MA");
        Iban::from_str("MC5811222000010123456789030").expect("valid iban for MC");
        Iban::from_str("MD24AG000225100013104168").expect("valid iban for MD");
        Iban::from_str("ME25505000012345678951").expect("valid iban for ME");
        Iban::from_str("MF8420041010050500013M02606").expect("valid iban for MF");
        Iban::from_str("MG4012341234123412341234123").expect("valid iban for MG");
        Iban::from_str("MK07250120000058984").expect("valid iban for MK");
        Iban::from_str("ML75A12312341234123412341234").expect("valid iban for ML");
        Iban::from_str("MQ5120041010050500013M02606").expect("valid iban for MQ");
        Iban::from_str("MR1300020001010000123456753").expect("valid iban for MR");
        Iban::from_str("MT84MALT011000012345MTLCAST001S").expect("valid iban for MT");
        Iban::from_str("MU17BOMM0101101030300200000MUR").expect("valid iban for MU");
        Iban::from_str("MZ97123412341234123412341").expect("valid iban for MZ");
        Iban::from_str("NC8420041010050500013M02606").expect("valid iban for NC");
        Iban::from_str("NE58NE0380100100130305000268").expect("valid iban for NE");
        Iban::from_str("NI92BAMC000000000000000003123123").expect("valid iban for NI");
        Iban::from_str("NL91ABNA0417164300").expect("valid iban for NL");
        Iban::from_str("NO9386011117947").expect("valid iban for NO");
        Iban::from_str("PF5720041010050500013M02606").expect("valid iban for PF");
        Iban::from_str("PK36SCBL0000001123456702").expect("valid iban for PK");
        Iban::from_str("PL61109010140000071219812874").expect("valid iban for PL");
        Iban::from_str("PM3620041010050500013M02606").expect("valid iban for PM");
        Iban::from_str("PS92PALS000000000400123456702").expect("valid iban for PS");
        Iban::from_str("PT50000201231234567890154").expect("valid iban for PT");
        Iban::from_str("QA58DOHB00001234567890ABCDEFG").expect("valid iban for QA");
        Iban::from_str("RE4220041010050500013M02606").expect("valid iban for RE");
        Iban::from_str("RO49AAAA1B31007593840000").expect("valid iban for RO");
        Iban::from_str("RS35260005601001611379").expect("valid iban for RS");
        Iban::from_str("SA0380000000608010167519").expect("valid iban for SA");
        Iban::from_str("SC18SSCB11010000000000001497USD").expect("valid iban for SC");
        Iban::from_str("SE4550000000058398257466").expect("valid iban for SE");
        Iban::from_str("SI56191000000123438").expect("valid iban for SI");
        Iban::from_str("SK3112000000198742637541").expect("valid iban for SK");
        Iban::from_str("SM86U0322509800000000270100").expect("valid iban for SM");
        Iban::from_str("SN15A12312341234123412341234").expect("valid iban for SN");
        Iban::from_str("ST68000100010051845310112").expect("valid iban for ST");
        Iban::from_str("SV62CENR00000000000000700025").expect("valid iban for SV");
        Iban::from_str("TD8960003000203710253860174").expect("valid iban for TD");
        Iban::from_str("TF2120041010050500013M02606").expect("valid iban for TF");
        Iban::from_str("TG53TG0090604310346500400070").expect("valid iban for TG");
        Iban::from_str("TL380080012345678910157").expect("valid iban for TL");
        Iban::from_str("TN5910006035183598478831").expect("valid iban for TN");
        Iban::from_str("TR330006100519786457841326").expect("valid iban for TR");
        Iban::from_str("UA213996220000026007233566001").expect("valid iban for UA");
        Iban::from_str("VG96VPVG0000012345678901").expect("valid iban for VG");
        Iban::from_str("WF9120041010050500013M02606").expect("valid iban for WF");
        Iban::from_str("XK051212012345678906").expect("valid iban for XK");
        Iban::from_str("YT3120041010050500013M02606").expect("valid iban for YT");
    }
}
