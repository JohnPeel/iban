use std::{env, path::PathBuf};

use quote::quote;
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
    //bban_bankid_start_offset: Option<usize>,
    //bban_bankid_stop_offset: Option<usize>,
    //bban_branchid_start_offset: Option<usize>,
    //bban_branchid_stop_offset: Option<usize>,
    //registry_edition: String,
    //country_sepa: String,
    //swift_official: String,
    //bban_checksum_start_offset: Option<usize>,
    //bban_checksum_stop_offset: Option<usize>,
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

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'|')
        .has_headers(true)
        .from_path("./registry.txt")
        .expect("failed to create csv reader for registry.txt");

    let pattern = Regex::new(r"(\d+)!(a|n|c|i)").expect("regex should be valid");

    let mut map = phf_codegen::Map::new();
    for record in reader.deserialize() {
        let Record {
            country_code,
            iban_format_swift,
            iban_length,
        } = record.expect("valid record");
        let captures = pattern
            .captures_iter(&iban_format_swift[2..])
            .map(|captures| {
                (
                    captures[1].parse::<usize>().unwrap(),
                    captures[2].parse::<char>().unwrap(),
                )
            })
            .map(|(len, char)| quote! { (#len, #char) });
        let captures = iban_format_swift[..2]
            .as_bytes()
            .iter()
            .map(|byte| (1usize, char::from(*byte)))
            .map(|(len, char)| quote! { (#len, #char) })
            .chain(captures);

        let value = quote! {
            (#iban_length, &[#(#captures),*])
        };
        map.entry(country_code, value.to_string().as_str());
    }

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    std::fs::write(
        out_path.join("countries.rs"),
        format!(
            "#[allow(clippy::type_complexity, clippy::unreadable_literal)]\nstatic COUNTRIES: ::phf::Map<&'static str, (usize, &'static [(usize, char)])> = {};\n",
            map.build()
        ),
    )
    .expect("failed to write countries file");
}
