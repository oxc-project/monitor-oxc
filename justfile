#!/usr/bin/env -S just --justfile

_default:
  @just --list -u

alias r := ready

init:
  cargo binstall cargo-watch taplo-cli -y

ready:
  cargo check

lint:
  cargo clippy

watch command:
  cargo watch --no-vcs-ignores -i 'repos' -x '{{command}}'
