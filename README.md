# whtkbrd

Forked from [KeySeeBee](https://github.com/TeXitoi/keyseebee)

[PCB schematic and layout](https://oshwlab.com/whtk/keeb1)

Using [Keyberon](https://github.com/TeXitoi/keyberon)

## Compiling and flashing

Install the complete toolchain and utils:

```shell
curl https://sh.rustup.rs -sSf | sh
rustup target add thumbv6m-none-eabi
rustup component add llvm-tools-preview
cargo install cargo-binutils
sudo apt-get install dfu-util
```

Compile:

```shell
cd firmware
cargo objcopy --bin keyseebee --release -- -O binary keyseebee.bin
```

To flash using dfu-util, first put the board in dfu mode by pressing
BOOT, pressing and releasing RESET and releasing BOOT. Then:

```shell
dfu-util -d 0483:df11 -a 0 -s 0x08000000:leave -D keyseebee.bin
```

The fist time, if the write fail, your flash might be protected. To
unprotect:

```shell
dfu-util -d 0483:df11 -a 0 -s 0x08000000:force:unprotect -D keyseebee.bin
```

Ideally both sides needs to be flashed, but for changes that only affect the layout it's strictly not needed as the side connected with the USB cable will determine the layout. In fact, you can have different layouts stored on each half, meaning you can switch back and forth between two layouts by moving the USB cable between the two halves.

## Configuration

Set keyboard layout in `firmware/src/main.rs`. [Available keycodes](https://github.com/TeXitoi/keyberon/blob/master/src/key_code.rs) for reference.