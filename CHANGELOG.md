## v0.1.7 (2023-07-04)

*  bumped package version for next development cycle
*  Add a test that we can create random ibans for all countries
*  Fix QA to match https://www.swift.com/node/9606
*  Fix QA to match https://www.swift.com/node/9606
*  added CHANGELOG file
*  added '--cfg docsrs' to docs.rs config in Cargo.toml


## v0.1.6 (2023-04-23)

*  bumped version for dev
*  Fix inconsistencies for DJ and KM
*  updated bban_format_swift to match changed iban_format_swift
*  extracted checksum calculation to it's own function
*  added ability to generate random ibans
*  fixed no_std build
*  moved rand functions under rand feature
*  added doc_auto_cfg to crate attributes


## v0.1.5 (2023-04-20)

*  introduced CharacterType to make generating IBANs easier
*  merged bban validation into iban population loop
*  cleaned up the end of Iban::from_str
*  converted chunk_str into a proper iter struct
*  fixed formatting
*  bumped version for cargo publish


## v0.1.4 (2023-04-18)

*  swapped Iban::from_str to Iban::parse in tests
*  updated code coverage action to use cargo-llvm-cov
*  implemented Clone explicitly to take advantage of Copy
*  created helper test functions to use in test_cases
*  updated documentation
*  bumped version for cargo publish


## v0.1.3 (2023-04-12)

*  fixed wrong version in README


## v0.1.2 (2023-04-12)

*  added workflows and more testing
*  added override for syn in Cargo.toml to fix minimal version
*  cleaned up workflows
*  moved to direct-minimal-versions
*  fixed code-coverage workflow
*  publish lcov report to codecov.io
*  added Bban type with bank, branch, and checksum getters
*  changed let-else to ok_or to decrease msrv from 1.65 to 1.60
*  added more information to README and added documentation
*  added Iban::parse inline forward to FromStr::from_str
*  changed README example to use "...".parse()
*  bumped version for cargo publish


## v0.1.1 (2023-04-10)

*  cleaned up and reorganized build script
*  changing required versions of dependencies to match actual requirements
*  bumping version in Cargo.toml for release

## v0.1.0 (2023-04-05)

*  initial commit
*  initial work on Iban type and parsing the registry.txt from php-iban
*  bumped version for cargo publish
