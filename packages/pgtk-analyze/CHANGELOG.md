# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/freshtonic/pgtk/releases/tag/pgtk-analyze-v0.1.0) - 2026-10-28

### Added

- *(pgtk-sql)* read U&"x" UESCAPE 'c' in each identifier position
- *(pgtk-sql)* refuse what the gram.y action checks of #55 refuse
- *(pgtk-psql)* parse psql meta-commands, \set, \unset and \if blocks
- *(pgtk-catalog)* [**breaking**] load PostgreSQL 18, and 19 beta with pg19-beta

### Fixed

- *(pgtk-analyze)* build at pg14 and pg15 again
- *(pgtk-sql)* refuse a U&"..." identifier with an incorrect escape
- *(pgtk-sql)* give the MERGE INSERT column list the items of INSERT
- *(pgtk-sql)* reject the 23 corpus statements that the strict check found
- *(pgtk-sql)* continue each string kind over a newline, and widen UESCAPE
- *(pgtk-sql)* give each gram.y Sconst every string spelling
- *(pgtk-sql)* parse OPERATOR(op) before a string literal
- *(pgtk-sql)* keep the CYCLE values and the indirections of gram.y
- *(pgtk-sql)* end the indirection of a parenthesized value at a star
- *(pgtk-sql)* keep the window frame bounds that frame_extent keeps
- *(pgtk-sql)* keep rows, qualified names and the JSON_TABLE path to gram.y
- *(pgtk-sql)* keep the ALTER and utility statement forms of gram.y
- *(pgtk-sql)* keep the CREATE statement forms of gram.y
- *(pgtk-sql)* reject the statements that a gram.y action rejects
- *(pgtk-sql)* parse a `)` before a set operator after ONLY and in GROUP BY
- *(pgtk-sql)* reject an empty JSON_TABLE column list
- *(pgtk-sql)* accept only the narrower forms gram.y wants in some places
- *(pgtk-sql)* keep lists and one-time clauses to the cardinality of gram.y
- *(pgtk-sql)* keep type names to the forms of gram.y Typename
- *(pgtk-sql)* give qualified_name, any_name and name_list their own types

### Other

- *(pgtk-analyze)* replay psql's submissions in the oracle harness
- define Rejection, Explanation and the recovering parse in the glossaries
- *(pgtk-analyze)* pin the corpus statements that pgtk-sql now rejects
- *(pgtk-analyze)* pin the event trigger argument that pgtk-sql now rejects
- *(pgtk-analyze)* add the describe, check_queries and target_version examples, and doc examples for the Analyzer
- put each README in the crate docs, turn on doctests, and add policy files and checks
- Add READMEs for the workspace and each crate, and configure release-plz
- Use recursa 0.1.1
- Document the build-override profile for pgtk-sql users
- Make the exported crates publishable
- Run every test in cargo test --workspace
- Vendor PostgreSQL once, at vendor/postgres
- Build the pgtk crates with Buck2
- Integrate the imported crates as pgtk-* workspace members
