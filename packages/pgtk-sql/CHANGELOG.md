# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/freshtonic/pgtk/releases/tag/pgtk-sql-v0.1.0) - 2026-10-28

### Added

- *(pgtk-sql)* read U&"x" UESCAPE 'c' in each identifier position
- *(pgtk-psql)* end query buffers and read COPY data as psql does
- *(pgtk-sql)* add a recovering document parse
- *(pgtk-sql)* refuse what the gram.y action checks of #55 refuse
- *(pgtk-psql)* parse psql meta-commands, \set, \unset and \if blocks

### Fixed

- *(pgtk-sql)* take a `)` before a string literal in DISTINCT ON and RETURNING WITH
- *(pgtk-sql)* refuse an unquoted current_schema in the column-name lists
- *(pgtk-sql)* read UESCAPE as parser.c does, after a Unicode string only
- *(pgtk-sql)* refuse an unquoted current_schema in each ColId rule
- *(pgtk-sql)* refuse a U&"..." identifier with an incorrect escape
- *(pgtk-sql)* give the MERGE INSERT column list the items of INSERT
- *(pgtk-sql)* lex identifiers and dollar tags as scan.l, and verify E'...' bytes
- *(pgtk-sql)* parse the statements of a BEGIN ATOMIC routine body
- *(pgtk-psql)* continue an escape string over a newline as psql does
- *(pgtk-sql)* reject the 23 corpus statements that the strict check found
- *(pgtk-sql)* continue each string kind over a newline, and widen UESCAPE
- *(pgtk-sql)* give each gram.y Sconst every string spelling
- *(pgtk-sql)* parse OPERATOR(op) before a string literal
- *(pgtk-sql)* take only constants as CYCLE mark values
- *(pgtk-sql)* parse the POSITION haystack as a b_expr
- *(pgtk-sql)* keep the CYCLE values and the indirections of gram.y
- *(pgtk-sql)* end the indirection of a parenthesized value at a star
- *(pgtk-sql)* keep the window frame bounds that frame_extent keeps
- *(pgtk-sql)* keep rows, qualified names and the JSON_TABLE path to gram.y
- *(pgtk-sql)* match the column target in the migration_lint example
- *(pgtk-sql)* read a ) before a set operator as a plain )
- *(pgtk-sql)* read the WITH of an option list once
- *(pgtk-sql)* keep the ALTER and utility statement forms of gram.y
- *(pgtk-sql)* keep the CREATE statement forms of gram.y
- *(pgtk-sql)* reject the statements that a gram.y action rejects
- *(pgtk-sql)* parse a `)` before a set operator after ONLY and in GROUP BY
- *(pgtk-sql)* reject an empty JSON_TABLE column list
- *(pgtk-sql)* accept only the narrower forms gram.y wants in some places
- *(pgtk-sql)* keep lists and one-time clauses to the cardinality of gram.y
- *(pgtk-sql)* keep type names to the forms of gram.y Typename
- *(pgtk-sql)* give qualified_name, any_name and name_list their own types
- *(pgtk-sql)* reject the function and operator signatures that gram.y rejects
- *(pgtk-sql)* reject the role specifications that gram.y rejects
- *(pgtk-sql)* accept only SCONST where gram.y wants Sconst
- *(pgtk-sql)* do not start a sampled dollar-quote tag with a digit

### Other

- remove the legacy import provenance check
- *(pgtk-sql)* run the Arbitrary round trip at pg19-beta again
- *(pgtk-sql)* ignore the Arbitrary round trip at pg19-beta
- *(pgtk-sql)* gate the strict tests of 15, 16 and 17 syntax by version
- *(pgtk-sql)* frame each recovering window with frame_window
- *(pgtk-sql)* run the BEGIN ATOMIC island test
- *(pgtk-analyze)* replay psql's submissions in the oracle harness
- *(pgtk-sql)* do not link the private UTF-8 check from sconst_bytes
- *(pgtk-sql)* check psql's submissions against the oracle, not the legacy ranges
- *(pgtk-sql)* check each Iconst position against the oracle at the int32 edges
- *(pgtk-sql)* fail each statement that the oracle rejects and pgtk-sql accepts
- *(pgtk-sql)* count the POSITION and CYCLE embedded tests
- *(pgtk-sql)* remove the round-trip entries of #60 and #61
- *(pgtk-sql)* count each multiple-clause check of insertSelectOptions under #55
- define Rejection, Explanation and the recovering parse in the glossaries
- *(pgtk-sql)* add the parse, version_check and migration_lint examples, and doc examples for Statement and parse_sql
- state the measured validity of Arbitrary statements
- *(pgtk-sql)* add the Arbitrary round trip, checked against the PostgreSQL oracle
- put each README in the crate docs, turn on doctests, and add policy files and checks
- Add READMEs for the workspace and each crate, and configure release-plz
- Use recursa 0.1.1
- Fix the two rustdoc warnings, and check the docs in CI
- Cancel superseded pull request runs, and cite pgtk-sql AGENTS.md
- Retire the pg-sql migration record
- Set up the agent docs for the monorepo layout
- Make every package pass cargo fmt and clippy
- Document the build-override profile for pgtk-sql users
- Make the exported crates publishable
- Fix the import provenance verifier for the pgtk layout
- Remove the nested CI workflows
- Vendor PostgreSQL once, at vendor/postgres
- Build the pgtk crates with Buck2
- Integrate the imported crates as pgtk-* workspace members
