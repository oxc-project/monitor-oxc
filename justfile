#!/usr/bin/env -S just --justfile

_default:
  @just --list -u

alias r := ready

init:
  cargo binstall cargo-watch taplo-cli -y

ready:
  cargo check
  cargo clippy
  pnpm install
  cargo run

lint:
  cargo clippy
