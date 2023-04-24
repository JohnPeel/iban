#![doc = include_str!("../README.md")]
#![cfg_attr(not(any(feature = "std", test)), no_std)]
#![warn(missing_docs)]

use core::{fmt, ops::Deref, str::FromStr};

use arrayvec::ArrayString;

mod util;
use util::{digits, ChunksExt as _, IteratorExt as _};

include!(concat!(env!("OUT_DIR"), "/countries.rs"));

const IBAN_MAX_LENGTH: usize = 34;

/// Represents an IBAN.
///
/// A valid International Bank Account Number (IBAN) is a bank account number that is internationally
/// recognized based on its country-specific format. An IBAN is used to facilitate international money
/// transfers and uniquely identifies the account held by a bank in a particular country. This struct
/// represents a valid IBAN and satisfies the length defined for that country, has a valid checksum and
/// has a Basic Bank Account Number (BBAN) format as defined in the IBAN registry.
///
/// # Construction
///
/// Use [`FromStr`](std::str::FromStr) to construct an `Iban` object from a string. If the provided string
/// does not meet the requirements of a valid IBAN, an error is returned. Once constructed, the `Iban` object
/// can be used to retrieve the country code, check digits, and BBAN.
///
/// # Formatting
///
/// Spaced formatting of the `Iban` can be obtained from the [`Display`](std::fmt::Display) implementation.
/// Electronic formatting can be obtained from the [`Debug`](std::fmt::Debug), [`Deref`](std::ops::Deref),
/// or [`AsRef`](std::convert::AsRef) implementations.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct Iban(ArrayString<IBAN_MAX_LENGTH>);

/// Represents the Basic Bank Account Number (BBAN) portion of an International Bank Account Number (IBAN).
///
/// The Bban struct provides methods to extract the bank identifier, branch identifier, and checksum (if available) from the BBAN.
///
/// If the BBAN does not contain a bank identifier, branch identifier or checksum, the respective methods will return None.
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
    /// Spaced formatting of the `Iban`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.as_ref().chunks::<4>().delimited(" ") {
            write!(f, "{chunk}")?;
        }

        Ok(())
    }
}

impl fmt::Display for Bban {
    /// Spaced formatting of the `Bban`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for chunk in self.as_ref().chunks::<4>().delimited(" ") {
            write!(f, "{chunk}")?;
        }

        Ok(())
    }
}

/// Represents the type of a character in an IBAN.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CharacterType {
    /// Digits (numeric characters 0 to 9 only)
    N,
    /// Upper case letters (alphabetic characters A-Z only)
    A,
    /// Upper and lower case alphanumeric characters (A-Z, a-z and 0-9)
    C,

    /// Upper case alphanemeric characters (A-Z and 0-9)
    ///
    /// Only used in IIBANs, as they are strict on casing.
    I,
    /// Specific character
    ///
    /// This is used for the country code.
    S(u8),
}

impl CharacterType {
    /// Returns true if `ch` is a member of the character type `self`.
    pub const fn contains(self, ch: u8) -> bool {
        match self {
            CharacterType::N => ch.is_ascii_digit(),
            CharacterType::A => ch.is_ascii_uppercase(),
            CharacterType::C => ch.is_ascii_alphanumeric(),
            CharacterType::I => ch.is_ascii_uppercase() || ch.is_ascii_digit(),
            CharacterType::S(expected) => ch == expected,
        }
    }

    /// Returns a random member of the character type `self`.
    #[cfg(feature = "rand")]
    pub fn rand<R: ?Sized + rand::Rng>(self, rng: &mut R) -> u8 {
        match self {
            CharacterType::N => rng.gen_range(b'0'..=b'9'),
            CharacterType::A => rng.gen_range(b'A'..=b'Z'),
            CharacterType::C => {
                let r = rng.gen_range(0..62);

                if r < 10 {
                    b'0' + r
                } else if r < 36 {
                    b'A' + r - 10
                } else {
                    b'a' + r - 36
                }
            }
            CharacterType::I => {
                let r = rng.gen_range(0..36);

                if r < 10 {
                    b'0' + r
                } else {
                    b'A' + r - 10
                }
            }
            CharacterType::S(expected) => expected,
        }
    }
}

/// An error that can occur when parsing an IBAN string.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ParseError {
    /// The country code of the IBAN is not composed of two ASCII letters.
    CountryCode,
    /// The check digits of the IBAN are not ASCII digits.
    CheckDigit,
    /// The IBAN contains a non-ASCII alphanumeric character.
    InvalidCharacter,
    /// The country of this IBAN is unknown.
    ///
    /// If you're sure that it should be known, please open an issue.
    UnknownCountry,
    /// The length of the IBAN does not match the expected length for the country.
    InvalidLength,
    /// The format of the BBAN does not match the expected format for the country.
    InvalidBban,
    /// The calculated checksum of the IBAN is invalid.
    WrongChecksum,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CountryCode => "invalid country code",
            Self::CheckDigit => "invalid check digit",
            Self::InvalidCharacter => "invalid character",
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

    /// Allows for obtaining a reference to the underlying `Iban` as a `&str`.
    ///
    /// This implementation returns the electronic-format representation of a IBAN.
    ///
    /// # Examples
    ///
    /// ```
    /// use iban::Iban;
    ///
    /// let iban: Iban = "FR1420041010050500013M02606".parse().unwrap();
    ///
    /// assert_eq!(&*iban, "FR1420041010050500013M02606");
    /// ```
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl Deref for Bban {
    type Target = str;

    /// Allows for obtaining a reference to the underlying `Bban` as a `&str`.
    ///
    /// This implementation returns the electronic-format representation of a BBAN.
    ///
    /// # Examples
    ///
    /// ```
    /// use iban::{Bban, Iban};
    ///
    /// let iban: Iban = "FR1420041010050500013M02606".parse().unwrap();
    /// let bban: Bban = iban.bban();
    ///
    /// assert_eq!(&*bban, "20041010050500013M02606");
    /// ```
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0[4..]
    }
}

impl AsRef<str> for Iban {
    /// Allows for obtaining a reference to the underlying `Iban` as a `&str`.
    ///
    /// This implementation returns the electronic-format representation of a IBAN.
    ///
    /// # Examples
    ///
    /// ```
    /// use iban::Iban;
    ///
    /// let iban: Iban = "FR1420041010050500013M02606".parse().unwrap();
    ///
    /// assert_eq!(iban.as_ref(), "FR1420041010050500013M02606");
    /// ```
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl AsRef<str> for Bban {
    /// Allows for obtaining a reference to the underlying `Bban` as a `&str`.
    ///
    /// This implementation returns the electronic-format representation of a BBAN.
    ///
    /// # Examples
    ///
    /// ```
    /// use iban::{Bban, Iban};
    ///
    /// let iban: Iban = "FR1420041010050500013M02606".parse().unwrap();
    /// let bban: Bban = iban.bban();
    ///
    /// assert_eq!(bban.as_ref(), "20041010050500013M02606");
    /// ```
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl FromStr for Iban {
    type Err = ParseError;

    /// Parses a string as an IBAN.
    ///
    /// This function attempts to parse the given string as an IBAN. If successful, it returns
    /// an `Iban` instance with the same value as the parsed string. Otherwise, it returns a
    /// [`ParseError`] indicating the reason for the failure.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut iban = ArrayString::<IBAN_MAX_LENGTH>::new();
        let mut characters = value
            .as_bytes()
            .iter()
            .copied()
            .filter(|byte| !byte.is_ascii_whitespace())
            .map(|b| b.to_ascii_uppercase());

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

        let country_code = &iban[..2];
        let &(expected_length, validation, ..) = COUNTRIES
            .get(country_code)
            .ok_or(ParseError::UnknownCountry)?;

        let mut validation = validation
            .iter()
            .flat_map(|(count, character_type)| (0..*count).map(move |_| character_type))
            .skip(4)
            .copied();

        for ch in characters {
            if !ch.is_ascii_alphanumeric() {
                return Err(ParseError::InvalidCharacter);
            }

            let character_type = validation.next().ok_or(ParseError::InvalidLength)?;
            if !character_type.contains(ch) {
                return Err(ParseError::InvalidBban);
            }

            iban.try_push(char::from(ch))
                .map_err(|_| ParseError::InvalidLength)?;
        }

        if validation.next().is_some() {
            return Err(ParseError::InvalidLength);
        }

        if expected_length != iban.len() {
            return Err(ParseError::InvalidLength);
        }

        if calculate_checksum(iban.as_bytes()) != 1 {
            return Err(ParseError::WrongChecksum);
        }

        Ok(Self(iban))
    }
}

impl Iban {
    /// Get the country code of the IBAN.
    ///
    /// Returns a string slice containing the two-letter country code at the beginning of the IBAN.
    #[inline]
    #[must_use]
    pub fn country_code(&self) -> &str {
        &self[0..2]
    }

    /// Get the check digits of the IBAN.
    ///
    /// Returns a string slice containing the two check digits immediately following the country code.
    #[inline]
    #[must_use]
    pub fn check_digits(&self) -> &str {
        &self[2..4]
    }

    /// Get the BBAN of the IBAN.
    ///
    /// Returns a `Bban` struct containing the basic bank account number (BBAN) portion of the IBAN.
    #[inline]
    #[must_use]
    pub const fn bban(&self) -> Bban {
        Bban(self.0)
    }

    /// Get the IBAN as a string slice.
    ///
    /// Returns a reference to the underlying string (electronic-format) that represents the IBAN.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self
    }

    /// Parse a string as an Iban.
    ///
    /// This method attempts to parse a string as an `Iban`. It returns a `Result`
    /// containing the parsed `Iban` if successful, or a [`ParseError`] if the string
    /// could not be parsed as an `Iban`.
    ///
    /// # Errors
    /// This method returns a `ParseError` for any of the following issues:
    /// * Country code format issues (see: `ParseError::CountryCode`)
    /// * Check digit format issues (see: `ParseError::CheckDigit`)
    /// * Invalid characters (see: `ParseError::InvalidCharacter`)
    /// * Over maximum IBAN length (see: `ParseError::TooLong`)
    /// * Unknown country (see: `ParseError::UnknownCountry`)
    /// * Invalid length (see: `ParseError::InvalidLength`)
    /// * Invalid BBAN format (see: `ParseError::InvalidBban`)
    /// * Checksum is wrong (see: `ParseError::WrongChecksum`)
    #[inline]
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        FromStr::from_str(s)
    }

    /// Generates a random IBAN for the specified `country_code` using the given `rng`.
    ///
    /// # Returns
    /// If successful, returns an `Iban` instance representing the generated IBAN.
    ///
    /// # Errors
    /// Returns a `ParseError` if the specified `country_code` is invalid or unknown.
    #[cfg(feature = "rand")]
    pub fn rand<R: ?Sized + rand::Rng>(
        country_code: &str,
        rng: &mut R,
    ) -> Result<Self, ParseError> {
        let mut iban = ArrayString::<IBAN_MAX_LENGTH>::new();
        let mut country_code = country_code.as_bytes().iter().map(u8::to_ascii_uppercase);

        for _ in 0..2 {
            let ch = country_code
                .next()
                .filter(u8::is_ascii_uppercase)
                .ok_or(ParseError::CountryCode)?;
            iban.push(char::from(ch));
        }

        if country_code.next().is_some() || iban.len() != 2 {
            return Err(ParseError::UnknownCountry);
        }

        iban.push_str("00");

        let &(expected_length, validation, ..) = COUNTRIES
            .get(&iban[..2])
            .ok_or(ParseError::UnknownCountry)?;

        let bban_chars = validation
            .iter()
            .flat_map(|(count, character_type)| (0..*count).map(move |_| character_type))
            .skip(4)
            .map(|character_type| char::from(character_type.rand(rng)));

        for character in bban_chars {
            iban.try_push(character)
                .map_err(|_| ParseError::InvalidLength)?;
        }

        debug_assert_eq!(iban.len(), expected_length);

        let check_digits = 98 - calculate_checksum(iban.as_bytes());
        #[allow(clippy::cast_possible_truncation)]
        let check_digits = [
            b'0' + (check_digits / 10) as u8,
            b'0' + (check_digits % 10) as u8,
        ];

        // TODO: Figure out a way to swap out the characters without unsafe.
        // SAFETY: All of the characters generated are ASCII, so there are no issues with character boundries.
        unsafe { &mut iban.as_bytes_mut()[2..4] }.copy_from_slice(&check_digits);

        Ok(Self(iban))
    }
}

impl Bban {
    /// Get the country code of the BBAN.
    ///
    /// Returns a string slice containing the two-letter country code of the BBAN.
    ///
    /// As `Bban` can only be constructed from a valid [`Iban`],
    /// this should always be a valid country code.
    #[inline]
    #[must_use]
    fn country_code(&self) -> &str {
        &self.0[0..2]
    }

    /// Get the bank identifier of the BBAN (if it has one).
    ///
    /// Returns an `Option` containing a string slice representing the bank identifier,
    /// or `None` if the BBAN does not have a bank identifier. The bank identifier's position
    /// and length are determined by the country's IBAN specification.
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
    ///
    /// Returns an `Option` containing a string slice representing the branch identifier,
    /// or `None` if the BBAN does not have a branch identifier. The branch identifier's position
    /// and length are determined by the country's IBAN specification.
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
    ///
    /// Returns an `Option` containing a string slice representing the checksum,
    /// or `None` if the BBAN does not have a checksum. The checksum's position
    /// and length are determined by the country's IBAN specification.
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

    /// Get the BBAN as a string slice.
    ///
    /// Returns a reference to the underlying string (electronic-format) that represents the BBAN.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self
    }
}

/// Calculates the checksum of an IBAN.
///
/// This function takes a valid IBAN string as input and returns the calculated
/// checksum as an unsigned 32-bit integer. The checksum is calculated by converting
/// the letters in the IBAN to digits, and then performing a series of modulus operations
/// on the resulting number.
///
/// Non-ASCII alphanumeric characters in the input will be ignored.
///
/// You can also use this method to generate the check digits for an IBAN.
/// Set the check digits to "00", then calculate the checksum and subtract that result from 98.
///
/// ```rust
/// use iban::Iban;
///
/// let original_iban   = "GB29NWBK60161331926819";
/// let zeroed_iban     = format!("{}00{}", &original_iban[..2], &original_iban[4..]);
///
/// let check_digits = 98 - iban::calculate_checksum(zeroed_iban.as_bytes());
///
/// assert_eq!(check_digits, 29);
///
/// let calculated_iban = format!("{}{:02}{}", &original_iban[..2], check_digits, &original_iban[4..]);
///
/// assert_eq!(original_iban, calculated_iban);
/// ```
pub fn calculate_checksum(iban: &[u8]) -> u32 {
    iban[4..]
        .iter()
        .chain(iban[..4].iter())
        .map(u8::to_ascii_uppercase)
        .filter(u8::is_ascii_alphanumeric)
        .flat_map(|byte| {
            if byte.is_ascii_digit() {
                digits(byte - b'0')
            } else {
                digits(byte - b'A' + 10)
            }
        })
        .fold(0u32, |checksum, byte| {
            let checksum = checksum * 10 + u32::from(byte);
            if checksum > 9_999_999 {
                checksum % 97
            } else {
                checksum
            }
        })
        % 97
}

#[cfg(test)]
mod tests {
    use core::{convert, fmt, ops};

    use test_case::test_case;

    use crate::{digits, Iban, ParseError};

    fn is_clone<T: Clone>(value: &T) {
        let _value = value.clone();
    }

    fn is_copy<T: Copy>(value: T) {
        let _value = value;
        let _other = value;
    }

    fn is_debug<T: fmt::Debug>(value: &T) {
        assert!(!format!("{value:?}").is_empty());
    }

    fn is_display<T: fmt::Display>(value: &T) {
        assert!(!format!("{value}").is_empty());
    }

    fn is_deref_str<T: ops::Deref<Target = str>>(value: &T) {
        let _value = value.deref();
    }

    fn is_asref_str<T: convert::AsRef<str>>(value: &T) {
        let _value = value.as_ref();
    }

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
        assert_eq!(iban.bban().as_str(), &original[4..]);
        assert_eq!(iban.as_str(), original);

        is_clone(&iban);
        is_copy(iban);
        is_debug(&iban);
        is_display(&iban);
        is_deref_str(&iban);
        is_asref_str(&iban);
    }

    #[test_case("1T4120041010050500013M02606", ParseError::CountryCode; "country code")]
    #[test_case("YTa120041010050500013M02606", ParseError::CheckDigit; "check digit")]
    #[test_case("YT412*041010050500013M02606", ParseError::InvalidCharacter; "invalid character")]
    #[test_case("SC18SSCB11010000000000001497USDABCD", ParseError::InvalidLength; "too long")]
    #[test_case("ZZ18SSCB11010000000000001497USD", ParseError::UnknownCountry; "unknown country")]
    #[test_case("AA110011123Z567891238", ParseError::InvalidLength; "invalid length")]
    #[test_case("YT4120041010050500013M02606", ParseError::WrongChecksum; "wrong checksum")]
    #[test_case("YT3120041010050500013M0260a", ParseError::InvalidBban; "invalid bban")]
    fn parse_error(iban: &str, expected_err: ParseError) {
        assert_eq!(Iban::parse(iban), Err(expected_err));

        is_clone(&expected_err);
        is_copy(expected_err);
        is_debug(&expected_err);
        is_display(&expected_err);
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
        assert_eq!(bban.as_str(), &original[4..]);

        assert_eq!(bban.bank_identifier(), bank);
        assert_eq!(bban.branch_identifier(), branch);
        assert_eq!(bban.checksum(), checksum);

        is_clone(&bban);
        is_copy(bban);
        is_debug(&bban);
        is_display(&bban);
        is_deref_str(&bban);
        is_asref_str(&bban);
    }

    #[cfg(feature = "rand")]
    #[test]
    fn random_iban() {
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::from_seed([0; 32]);
        let iban = Iban::rand("GB", &mut rng).expect("generates random (seeded) iban");

        assert_eq!(&*iban, "GB82KIBV70634724101729");

        assert_eq!(iban.country_code(), "GB");
        assert_eq!(iban.check_digits(), "82");

        let bban = iban.bban();
        assert_eq!(bban.bank_identifier(), Some("KIBV"));
        assert_eq!(bban.branch_identifier(), Some("706347"));
        assert_eq!(bban.checksum(), None);
    }
}
