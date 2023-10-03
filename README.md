# Grafbase on Deno

Simple test of Grafbase on Deno. This repo only contains the barebone engine with `@graphql` & `@openapi` directives (no auth, caching, `@model`, etc.)

Structure:

- The `app` crate generates the actual wasm published on Deno.
- The `parser` crate generates a JSON blob used by the engine as its configuration from the a schema file.

Setup:

- Use the nix flake / install `deno`, rustup and `cargo-make`.
- `cargo make install` to add wasm32 target & install deno deployctl
- `cargo make deploy` will:
    1. parse `schema.graphql`
    2. build wasm app for that schema
    3. deploy it to Deno (requires `DENO_PROJECT` & `DENO_DEPLOY_TOKEN`)

