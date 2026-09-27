# lykil-studio

Lykil Studio: manage lykil keyboards. built on [Aurea](https://github.com/SkuldNorniern/aurea), talks to the keyboard with LCP through `lykil-device`.

```sh
cargo run
```

no files and no JSON: the keyboard describes itself over LCP (name, layers, keys with position, size and matrix cell), so any keyboard running lykil works.

what it shows now:

- the connected keyboard: name, layers, keys, matrix size, LCP version
- live diagnostics once per second: uptime, scans, transitions, last reset, watchdog resets, storage, and healthy or not
- the keyboard as it describes itself, keys light up while they are down (matrix read every 20 ms). a keyboard without key positions is drawn as its raw matrix

it finds the keyboard by itself and comes back after an unplug. close VIA while it runs: every program with the raw HID interface open gets every answer.

later: editing bindings, layers, diagnostics history.
