# Internal ICP dependency investigation

Internal record only, 14 October 2026. The user restricted upstream reports to
DFINITY repositories. Do not publish this as an issue: the conflict belongs to a
third-party library, not DFINITY imcp2. That library is not used by the alpha.

Adding ic-rmcp to a canister using `ic-cdk = "=0.20.3"` fails dependency
resolution before compilation. The dependency graph requires two incompatible
versions of `ic-cdk-executor`, which intentionally share a Cargo `links` value.

## Reproduction

Rust and Cargo 1.95.0, macOS aarch64. An otherwise empty library is sufficient:

```toml
[package]
name = "ic-rmcp-cdk-020-repro"
version = "0.0.0"
edition = "2021"

[dependencies]
ic-cdk = "=0.20.3"
ic-rmcp = { git = "https://github.com/ByteSmithLabs/ic-rmcp", rev = "ca7961318d8168dd189dfc2bbe2b508b3cebdc6d" }
```

Create `src/lib.rs` as an empty file and run:

```sh
cargo +1.95.0 generate-lockfile
```

The reproduction also fails with `--offline` when the sources/index entries
are cached, ruling out a network failure as the cause. Cargo exits with code 101.

## Dependency conflict

The application requires `ic-cdk 0.20.3`, which uses `ic-cdk-executor 2.0.0`.
The ic-rmcp dependency on the `v9.3.1-port-wasm-ic` jsonwebtoken fork resolves
to commit `5a913f92`, requiring `ic-cdk ^0.18.5` and executor `^1.0.0`.
The ic-rmcp crate also directly requires `ic-cdk 0.18`.

Cargo reports:

```text
error: failed to select a version for `ic-cdk-executor`.
    ... required by package `ic-cdk v0.18.5`
    ... required by package `jsonwebtoken v9.3.1`
    ... required by package `ic-rmcp v0.1.0`

Only one package in the dependency graph may specify the same links value.
```

The pinned [ic-rmcp manifest](https://github.com/ByteSmithLabs/ic-rmcp/blob/ca7961318d8168dd189dfc2bbe2b508b3cebdc6d/ic-rmcp/Cargo.toml)
shows both dependencies. This is a runtime compatibility issue; removing the
executor's `links` safeguard would not be an appropriate fix.

## Requested direction

Is support for ic-cdk 0.20 planned? Updating the direct CDK requirement alone
will leave the JWT dependency on the older runtime. A compatible JWT fork, or
making that integration optional if the architecture permits it, appears necessary
as well. Neither change has been validated here as a complete fix.

## Duplicate check

The repository's returned public issue listing on 14 October 2026 contained
closed issues 1–3, including a README dependency-name correction, but no matching
runtime-version report. Recheck before posting.
