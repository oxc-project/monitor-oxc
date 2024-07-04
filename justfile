#!/usr/bin/env -S just --justfile

_default:
  @just --list -u

alias r := ready

init:
  cargo binstall cargo-watch taplo-cli -y

ready:
  cargo check
  pnpm install
  cargo run
  pnpm run test

lint:
  cargo clippy
