# lykil-studio

Lykil Studio: manage lykil keyboards. built on [Aurea](https://github.com/SkuldNorniern/aurea), talks to the keyboard with LCP through `lykil-device`.

```sh
cargo run -- ../lykil-k8pro     # with the keyboard project: keys drawn from layout.tav
cargo run                       # without: the raw matrix as a grid
```

what it shows now:

- the connected keyboard: name, layers, keys, matrix size, LCP version
- live diagnostics once per second: uptime, scans, transitions, last reset, watchdog resets, storage, and healthy or not
- the keyboard, keys light up while they are down (matrix read every 20 ms)

it finds the keyboard by itself and comes back after an unplug. close VIA while it runs: every program with the raw HID interface open gets every answer.

later: editing bindings, layers, diagnostics history.
