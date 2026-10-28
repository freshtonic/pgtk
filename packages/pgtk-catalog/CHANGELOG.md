# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/freshtonic/pgtk/releases/tag/pgtk-catalog-v0.1.0) - 2026-10-28

### Added

- *(pgtk-catalog)* [**breaking**] load PostgreSQL 18, and 19 beta with pg19-beta

### Fixed

- *(pgtk-catalog)* build the describe_schema example with the non-exhaustive enums

### Other

- *(pgtk-catalog)* add the describe_schema and snapshot examples, and a doc example for load_async
- put each README in the crate docs, turn on doctests, and add policy files and checks
- Add READMEs for the workspace and each crate, and configure release-plz
- Set up the agent docs for the monorepo layout
- Make the exported crates publishable
- Remove the nested CI workflows
- Run every test in cargo test --workspace
- Vendor PostgreSQL once, at vendor/postgres
- Build the pgtk crates with Buck2
- Integrate the imported crates as pgtk-* workspace members
