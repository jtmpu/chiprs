# chip-8 emulator, assembler and disassembler

```
cargo run --bin assembly -- asm --input assembly/examples/simple.asm --output test.bin
```

```
cargo run --bin emulator -- -v -d -f test.bin -t 40
```
