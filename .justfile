default: build

build:
    wasm-pack build --target web
    cargo doc --no-deps --open

test:
    cargo test --workspace --features sanitizehtml
    wasm-pack test --headless --firefox --features sanitizehtml
    #wasm-pack test --chrome  --headless

clean:
    cargo clean

push:
    cargo bump minor
    git add .
    git commit -m "Bump minor version"
    git push origin main

push-core:
    cd dappr-core
    cargo bump minor
    git add .
    git commit -m "Bump minor version"
    git push origin main
    cd ..

push-macros:
    cd dappr-macros
    cargo bump minor
    git add .
    git commit -m "Bump minor version"
    git push origin main
    cd ..

push-all: push push-core push-macros

[working-directory: 'examples/hello_dappr']
@example-hello:
    wasm-pack build --target web --release
    trunk serve --release

[working-directory: 'examples/html']
@example-html:
    wasm-pack build --target web --release
    trunk serve --release

[working-directory: 'examples/button']
@example-button:
    wasm-pack build --target web --release
    trunk serve --release

[working-directory: 'examples/text']
@example-text:
    wasm-pack build --target web --release
    trunk serve --release

[working-directory: 'examples/lifecycle']
@example-lifecycle:
    wasm-pack build --target web --release
    trunk serve --release
