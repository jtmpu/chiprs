# chip-8 emulator, assembler and disassembler

## General

* Assembly - binary to assemble and disassemble chip-8
* Emulator - execute chip-8 code

## TODO

Simple smaller stuff

- [ ] Support hex digits in instructions (0xff e.g.)
- [ ] Refactor parser.rs and clean it up
- [ ] Add more of the simpler instructions
- [ ] Headless emulator

Bigger stuff

- [x] Support key press from ratatui <-> emulator
- [x] Find a simple way to separate and control ticks for emulator and rendering
- [x] Diagnostics UI in ratatui
  - Pause, step, (maybe step back?)
  - Show registries, stacks
  [ ] Better diagnostics
  - allow read/write of memory 
  - prettier views
- [ ] Diagnostics CLI
- [ ] Multi-file support in assembly
- [ ] Sound playback in TUI client

## Examples

![current](https://github.com/jtmpu/chiprs/assets/20316416/dd3b7dc3-ead2-4ba1-b04c-710effcc6c6c)

![bild](https://github.com/jtmpu/chiprs/assets/20316416/64e6a7e6-1cef-46bc-becd-75c0088cb9ca)


Pipe outputs of the asm/disasm operations into each other, or emulator to execute the binary later on

```
user@rust:~/rust/chiprs$ cargo run --bin assembly -- asm -i assembly/examples/abort.asm | cargo run --bin assembly -- disasm | cargo run --bin assembly -- asm | cargo run --bin emulator
Emulator state:

Next instruction: Err(InvalidOpcode)

PC: 0210
SP: 00

regs:  r0:00 r1:0a r2:14 r3:00 r4:00 r5:00 r6:00 r7:00 r8:00 r9:00 r10:00 r11:00 r12:00 r13:00 r14:00 r15:00
```

```
user@rust:~/rust/chiprs$ cargo run --bin assembly -- asm --input assembly/examples/simple.asm --output test.bin
```

```
user@rust:~/rust/chiprs$ cargo run --bin emulator -- -v -d -f test.bin -t 40
```
