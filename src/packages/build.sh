#!/usr/bin/env sh
# packages/build.sh
#
# Builds every native package in release mode and copies its library into
# packages/<name>/lib/, where `liphia install` downloads it from.
#
#   sh src/packages/build.sh

set -e
packages="$(cd "$(dirname "$0")" && pwd)"
workspace="$(dirname "$packages")"

case "$(uname -s)" in
    Darwin) prefix="lib"; suffix=".dylib" ;;
    *)      prefix="lib"; suffix=".so" ;;
esac

cd "$workspace"
for name in db num stats learn; do
    echo "[build] liphia_package_$name"
    cargo build -p "liphia_package_$name" --release
    mkdir -p "$packages/$name/lib"
    cp "target/release/${prefix}liphia_package_${name}${suffix}" "$packages/$name/lib/"
done
echo "[build] done: libraries copied to packages/<name>/lib/"
