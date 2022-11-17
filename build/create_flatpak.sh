#!/bin/sh
VER=$(cat ../Cargo.toml | grep "version"  | awk -F"\"" '{print $2}' | head -n 1)
flatpak-builder --repo=repo --force-clean build-dir github.flatpak.icy_term.yml
flatpak build-bundle repo icy_term_$VER.flatpak github.flatpak.IcyTerm
rm -rf build-dir repo .flatpak-builder
