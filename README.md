# s3-server

[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]
[![Docs][docs-badge]][docs-url]
![CI][ci-badge]

[crates-badge]: https://img.shields.io/crates/v/s3-server.svg
[crates-url]: https://crates.io/crates/s3-server
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[docs-badge]: https://docs.rs/s3-server/badge.svg
[docs-url]: https://docs.rs/s3-server/
[ci-badge]: https://github.com/datenlord/s3-server/workflows/CI/badge.svg

An experimental generic S3 server

## Install

```shell
git clone https://github.com/datenlord/s3-server
cd s3-server
cargo install --features binary --path .
s3-server --help
```

```
s3-server 0.1.0-dev

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
