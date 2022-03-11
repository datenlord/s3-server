# s3-server

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Docs][docs-badge]][docs-url]
![CI][ci-badge]
[![Unsafe Forbidden][unsafe-forbidden-badge]][unsafe-forbidden-url]

[crates-badge]: https://img.shields.io/crates/v/s3-server.svg
[crates-url]: https://crates.io/crates/s3-server
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[docs-badge]: https://docs.rs/s3-server/badge.svg
[docs-url]: https://docs.rs/s3-server/
[ci-badge]: https://github.com/datenlord/s3-server/workflows/CI/badge.svg
[unsafe-forbidden-badge]: https://img.shields.io/badge/unsafe-forbidden-success.svg
[unsafe-forbidden-url]: https://github.com/rust-secure-code/safety-dance/

An experimental generic S3 server

## Install

From cargo:

```
cargo install s3-server --features binary
```

From source:

```shell
git clone https://github.com/datenlord/s3-server
cd s3-server
cargo install --features binary --path .
s3-server --help
```

## Usage

```
s3-server 0.2.0

USAGE:
    s3-server [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --fs-root <fs-root>           [default: .]
        --host <host>                 [default: localhost]
        --port <port>                 [default: 8014]
        --access-key <access-key>    
        --secret-key <secret-key>
```

## Debug

Set environment variable `RUST_LOG` to `s3_server=debug`
