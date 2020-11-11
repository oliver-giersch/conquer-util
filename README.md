# conquer-util

Utilities for lock-free and concurrent programming.

[![Build Status](https://travis-ci.org/oliver-giersch/conquer-util.svg?branch=master)](
https://travis-ci.org/oliver-giersch/conquer-util)
[![Latest version](https://img.shields.io/crates/v/conquer-util.svg)](https://crates.io/crates/conquer-util)
[![Documentation](https://docs.rs/conquer-util/badge.svg)](https://docs.rs/conquer-util)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/oliver-giersch/conquer-util)
[![Rust 1.36+](https://img.shields.io/badge/Rust-1.36.0-orange.svg)](
https://www.rust-lang.org)

## Usage

Add this to your `Cargo.toml` and adjust the selected features as required.

```toml
[dependencies.conquer-util]
version = "0.3.0"
features = ["align", "back-off", "tls"] # enables all features
```

## Minimum Supported Rust Version (MSRV)

The minimum supported Rust version for this crate is 1.36.0.

## Cargo Features

This crate offers fine-grained control over its contents through cargo feature
flags, from `#![no_std]` compatibility to selection of which utilities and
dependencies will be compiled.
For complete information see the crate's
[documentation](https://docs.rs/conquer-util).

## License

`conquer-util` is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
