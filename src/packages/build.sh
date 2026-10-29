#!/usr/bin/env sh
# packages/build.sh
#
# Builds native packages in release mode and copies each library into
# packages/<n>/lib/, where `liphia install` downloads it from.
#
# A package is native when its package.toml has a [native] section; the
# library name is the `lib` key of that section. With no arguments every
# native package is built; otherwise only the names given.
#
#   sh src/packages/build.sh
#   sh src/packages/build.sh crypto num

set -e
packages="$(cd "$(dirname "$0")" && pwd)"
workspace="$(dirname "$packages")"

case "$(uname -s)" in
    Darwin) prefix="lib"; suffix=".dylib" ;;
    *)      prefix="lib"; suffix=".so" ;;
esac

# Prints the `lib` value of the [native] section, or nothing when the
# package.toml has no such section.
native_lib() {
    awk '
        /^\[[A-Za-z0-9_.-]+\][[:space:]]*$/ { in_native = ($0 ~ /^\[native\]/); next }
        in_native && /^lib[[:space:]]*=/ { split($0, a, "\""); print a[2]; exit }
    ' "$1"
}

if [ "$#" -gt 0 ]; then
    names="$*"
else
    names=""
    for dir in "$packages"/*/; do
        [ -f "${dir}package.toml" ] || continue
        if [ -n "$(native_lib "${dir}package.toml")" ]; then
            names="$names $(basename "$dir")"
        fi
    done
    if [ -z "$names" ]; then
        echo "no native packages found in $packages" >&2
        exit 1
    fi
fi

cd "$workspace"
for name in $names; do
    toml="$packages/$name/package.toml"
    if [ ! -f "$toml" ]; then
        echo "no package '$name' (missing $toml)" >&2
        exit 1
    fi
    lib="$(native_lib "$toml")"
    if [ -z "$lib" ]; then
        echo "not a native package: $name" >&2
        exit 1
    fi

    echo "[build] $lib"
    cargo build -p "$lib" --release
    mkdir -p "$packages/$name/lib"
    cp "target/release/${prefix}${lib}${suffix}" "$packages/$name/lib/"
done
echo "[build] done: libraries copied to packages/<n>/lib/"