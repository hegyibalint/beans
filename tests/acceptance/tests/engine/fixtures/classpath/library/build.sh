#!/usr/bin/env sh
set -e
cd "$(dirname "$0")"

rm -rf build
mkdir -p build/classes
javac --release 17 -d build/classes src/com/example/library/CompiledLibrary.java
jar --create --file compiled-library.jar -C build/classes .
