# Dappr

Data Apps made easy (in Rust)

## Dependencies and useful tools

Dappr's workflow employs the following tools:

- [wasm-pack](https://rustwasm.github.io/wasm-pack/) ([repo](https://github.com/rustwasm/wasm-pack)) to build the library
- [trunk](https://trunkrs.dev/) ([repo](https://github.com/trunk-rs/trunk)) to run exampels and dapps locally
- [just](https://just.systems/) ([repo](https://github.com/casey/just) to automate builds

## Build

Dappr ships wih a .justfile, so you just run `just` as *build* is the default recipe:

```bash
just
```

## Run examples

Using `just` all examples recipes start with *example-* and use `trunk` to run.
List all recipes and select the example you want

```bash
just --list
```

For example:

```bash
just example-hello
```

## License

Copyright (c) 2024-2026 Davide Del Papa under the terms of the MIT License (see LICENSE file attached).
