default: build

build:
    wasm-pack build --target web
    cargo doc --no-deps --open

test:
    cargo test --workspace --features sanitizehtml
    wasm-pack test --headless --firefox --features sanitizehtml
    #wasm-pack test --headless --chrome --features sanitizehtml

clean:
    cargo clean

push:
    cargo bump minor
    git add .
    git commit -m "Bump minor version"
    git push origin main

[working-directory: 'dappr-core']
push-core:
    cargo bump minor
    git add .
    git commit -m "Bump minor version"
    git push origin main

[working-directory: 'dappr-macros']
push-macros:
    cargo bump minor
    git add .
    git commit -m "Bump minor version"
    git push origin main

push-all: push push-core push-macros

[working-directory: 'examples/hello_dappr']
@example-hello:
    trunk serve --release

[working-directory: 'examples/html']
@example-html:
    trunk serve --release

[working-directory: 'examples/button']
@example-button:
    trunk serve --release

[working-directory: 'examples/text']
@example-text:
    trunk serve --release

[working-directory: 'examples/lifecycle']
@example-lifecycle:
    trunk serve --release

[working-directory: 'examples/layout']
@example-layout:
    trunk serve --release

[working-directory: 'examples/inputs']
@example-inputs:
    trunk serve --release
