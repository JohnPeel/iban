use std::{cmp::min, env, path::PathBuf};

use quote::{format_ident, quote};
use regex::Regex;

#[derive(Debug, serde::Deserialize)]
struct Record {
    country_code: String,
    //country_name: String,
    //domestic_example: String,
    //bban_example: String,
    //bban_format_swift: String,
    //bban_format_regex: String,
    //bban_length: usize,
    //iban_example: String,
    iban_format_swift: String,
    //iban_format_regex: String,
    iban_length: usize,
    bban_bankid_start_offset: Option<usize>,
    bban_bankid_stop_offset: Option<usize>,
    bban_branchid_start_offset: Option<usize>,
    bban_branchid_stop_offset: Option<usize>,
    //registry_edition: String,
    //country_sepa: String,
    //swift_official: String,
    bban_checksum_start_offset: Option<usize>,
    bban_checksum_stop_offset: Option<usize>,
    //country_code_iana: String,
    //country_code_iso3166_1_alpha2: String,
    //parent_registrar: String,
    //currency_iso4217: String,
    //central_bank_url: String,
    //central_bank_name: String,
    //membership: String,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=registry.txt");

    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_RAND");
    if env::var("CARGO_FEATURE_RAND").is_ok() {
        println!("cargo:warning=The `rand` feature flag has been deprecated. Use `rand_0_8` or `rand_0_9`.");
    }

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(true)
        .from_path("./registry.txt")
        .expect("failed to create csv reader for registry.txt");

    // TODO: `\d+(a|n|c|i)` is also valid, and specifies a maximum length, rather than a fixed length.
    let pattern = Regex::new(r"(\d+)!(a|n|c|i)").expect("regex should be valid");

    let countries = reader
        .deserialize()
        .map(|record| record.expect("valid record"))
        .map(
            |Record {
                 country_code,
                 iban_format_swift,
                 iban_length,
                 bban_bankid_start_offset,
                 bban_bankid_stop_offset,
                 bban_branchid_start_offset,
                 bban_branchid_stop_offset,
                 bban_checksum_start_offset,
                 bban_checksum_stop_offset,
             }| {
                let captures = pattern
                    .captures_iter(&iban_format_swift[2..])
                    .map(|captures| {
                        (
                            captures[1].parse::<usize>().unwrap(),
                            format_ident!(
                                "{}",
                                captures[2]
                                    .parse::<char>()
                                    .unwrap()
                                    .to_ascii_uppercase()
                                    .to_string()
                            ),
                        )
                    })
                    .map(|(len, char)| quote! { (#len, CharacterType::#char) });
                let captures = iban_format_swift.as_bytes()[..2]
                    .iter()
                    .map(|byte| (1usize, byte.to_ascii_uppercase()))
                    .map(|(len, char)| quote! { (#len, CharacterType::S(#char)) })
                    .chain(captures);

                let bankid_offset = if let (Some(start), Some(end)) =
                    (bban_bankid_start_offset, bban_bankid_stop_offset)
                {
                    quote! { Some((#start, #end + 1)) }
                } else {
                    quote! { None }
                };

                let branch_offset = if let (Some(start), Some(end)) =
                    (bban_branchid_start_offset, bban_branchid_stop_offset)
                {
                    quote! { Some((#start, #end + 1)) }
                } else {
                    quote! { None }
                };

                let checksum_offset = if let (Some(start), Some(end)) =
                    (bban_checksum_start_offset, bban_checksum_stop_offset)
                {
                    quote! { Some((#start, #end + 1)) }
                } else {
                    quote! { None }
                };

                (
                    country_code,
                    quote! {
                        (
                            #iban_length,
                            &[#(#captures),*],
                            #bankid_offset,
                            #branch_offset,
                            #checksum_offset,
                        )
                    },
                )
            },
        )
        .collect::<Vec<_>>();

    let mut map = phf_codegen::Map::new();
    for (key, value) in &countries {
        map.entry(key.as_str(), value.to_string().as_str());
    }
    let countries = map.build();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    std::fs::write(
        out_path.join("countries.rs"),
        format!(
            "#[allow(clippy::type_complexity, clippy::unreadable_literal, clippy::identity_op)]\nstatic COUNTRIES: ::phf::Map<&'static str, (usize, &'static [(usize, CharacterType)], Option<(usize, usize)>, Option<(usize, usize)>, Option<(usize, usize)>)> = {countries};\n",
        ),
    )
    .expect("failed to write countries file");

    regex();
}

#[derive(Debug, serde::Deserialize)]
struct RegexRecord {
    country_code: String,
    iban_format_regex: String,
    iban_format_swift: String,
}

fn swift_to_regex(v: &str, len: usize) -> String {
    match v {
        "N" => {
            format!("[0-9]{{{}}}", len)
        }
        "I" => {
            format!("[A-Z0-9]{{{}}}", len)
        }
        "C" => {
            format!("[A-Z0-9]{{{}}}", len)
        }
        "A" => {
            format!("[A-Z]{{{}}}", len)
        }
        _ => "".to_string(),
    }
}

const REGEX_SPACE: &str = r#" "#;

fn regex() {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(true)
        .from_path("./registry.txt")
        .expect("failed to create csv reader for registry.txt");

    let pattern = Regex::new(r"(\d+)!(a|n|c|i)").expect("regex should be valid");

    let countries = reader
        .deserialize()
        .map(|record| record.expect("valid record"))
        .flat_map(
            |RegexRecord {
                 country_code,
                 iban_format_regex,
                 iban_format_swift,
             }| {
                let captures = pattern
                    .captures_iter(&iban_format_swift[2..])
                    .map(|captures| {
                        (
                            captures[1].parse::<usize>().unwrap(),
                            format!(
                                "{}",
                                captures[2]
                                    .parse::<char>()
                                    .unwrap()
                                    .to_ascii_uppercase()
                                    .to_string()
                            ),
                        )
                    });

                let buf = captures.collect::<Vec<(usize, String)>>();

                let blocks = buf.clone().into_iter().fold(
                    String::from(format!("{}{}", &country_code, REGEX_SPACE)),
                    |mut acc, (len, v)| {
                        let regex = swift_to_regex(v.as_str(), len);

                        acc.push_str(&regex);
                        acc.push_str(REGEX_SPACE);

                        acc
                    },
                );

                let space = buf.clone().into_iter().fold(
                    (String::from(country_code.clone()), 2),
                    |(mut acc, mut len), (k, v)| {
                        let mut section_length = k;
                        while section_length > 0 {
                            let remain = min(4 - len % 4, section_length);
                            let regex = swift_to_regex(v.as_str(), remain);

                            acc.push_str(&regex);

                            section_length -= remain;
                            len += remain;
                            if len % 4 == 0 {
                                acc.push_str(REGEX_SPACE);
                            }
                        }

                        (acc, len)
                    },
                );

                vec![
                    iban_format_regex
                        .trim_start_matches("^")
                        .trim_end_matches("$")
                        .to_string(),
                    space.0.trim_end_matches(REGEX_SPACE).to_string(),
                    blocks.trim_end_matches(REGEX_SPACE).to_string(),
                ]
            },
        );

    let regexes = countries.collect::<Vec<_>>();

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    std::fs::write(
        out_path.join("regex.rs"),
        format!(
            "const REGEXES: [&'static str; {}] = [{}];\n",
            regexes.len(),
            regexes
                .iter()
                .map(|r| format!("r#\"{}\"#", r))
                .collect::<Vec<String>>()
                .join(",\n")
        ),
    )
    .expect("failed to write regex file");
}
