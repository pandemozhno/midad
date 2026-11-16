#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"

OUT=build
mkdir -p "$OUT"

echo "→ cargo build --release"
cargo build --release

MIDC="./target/release/midc"
[ -x "$MIDC" ] || { echo "нет $MIDC"; exit 1; }

echo
echo "→ @flow hello (en) → C"
$MIDC examples/hello.en.mid --lang en --backend c  -o "$OUT/hello_en.c"
echo "→ @flow hello (en) → JS"
$MIDC examples/hello.en.mid --lang en --backend js -o "$OUT/hello_en.js"

echo "→ @flow hello (ru) → C"
$MIDC examples/hello.ru.mid --lang ru --backend c  -o "$OUT/hello_ru.c"

echo "→ @kernel hello → C"
$MIDC examples/hello_kernel.en.mid --lang en --backend c -o "$OUT/hello_kernel.c"

echo "→ @flow counter → C"
$MIDC examples/counter.en.mid --lang en --backend c -o "$OUT/counter.c"

echo "→ @flow counter → JS"
$MIDC examples/counter.en.mid --lang en --backend js -o "$OUT/counter.js"

echo
echo "→ сборка @flow hello (native)"
gcc -O0 -o "$OUT/hello_en" "$OUT/hello_en.c"
"$OUT/hello_en"

echo
echo "→ сборка @flow hello (native, ru)"
gcc -O0 -o "$OUT/hello_ru" "$OUT/hello_ru.c"
"$OUT/hello_ru"

echo
echo "→ проверка @kernel hello (freestanding)"
gcc -c -ffreestanding -nostdlib -fno-pic -fno-stack-protector -o "$OUT/hello_kernel.o" "$OUT/hello_kernel.c"
ld -m elf_x86_64 -T kernel/linker.ld -o "$OUT/hello_kernel.elf" "$OUT/hello_kernel.o"
objcopy -O binary "$OUT/hello_kernel.elf" "$OUT/hello_kernel.bin"
echo "    @kernel hello → $OUT/hello_kernel.bin ($(stat -c%s "$OUT/hello_kernel.bin") байт)"

echo
echo "→ counter (JS, node)"
node "$OUT/counter.js"

echo
echo "OK — все примеры скомпилированы и запущены"