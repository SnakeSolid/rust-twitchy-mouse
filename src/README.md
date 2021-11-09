## Twitching Mouse

Mouse example for STM32F103. Emulate USB mouse which moves every 10 seconds. Device can be used to
prevent computer locking.

## Build

Project can be build using cargo command:

```sh
cargo build --release
```

## Note

To write firmware to Chinese clones of stm32, it's necessarily to use configuration file `stm32f1x.cfg` with changed
`coreid`. Start `openocd` with custom configuration:

```sh
openocd -f /usr/share/openocd/scripts/interface/stlink-v2.cfg -f stm32f1x.cfg
```

Start remote GBD session in `gdb-multiarch`:

```sh
gdb-multiarch temperature-dht22
```

Following command can be used in GDB session co connect to `openocd` and write firmware

```
target remote :3333
monitor arm semihosting enable
load
continue
```
