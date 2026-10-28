# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/freshtonic/pgtk/releases/tag/pgtk-psql-v0.1.0) - 2026-10-28

### Added

- *(pgtk-psql)* end query buffers and read COPY data as psql does
- *(pgtk-psql)* parse psql meta-commands, \set, \unset and \if blocks

### Fixed

- *(pgtk-sql)* lex identifiers and dollar tags as scan.l, and verify E'...' bytes
- *(pgtk-psql)* continue an escape string over a newline as psql does

### Other

- *(pgtk-sql)* check psql's submissions against the oracle, not the legacy ranges
- put each README in the crate docs, turn on doctests, and add policy files and checks
- Add READMEs for the workspace and each crate, and configure release-plz
- Make every package pass cargo fmt and clippy
- Make the exported crates publishable
- Vendor PostgreSQL once, at vendor/postgres
- Build the pgtk crates with Buck2
- Integrate the imported crates as pgtk-* workspace members
