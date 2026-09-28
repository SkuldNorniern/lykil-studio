# lykil-studio

Lykil Studio: manage lykil keyboards. built on [Aurea](https://github.com/SkuldNorniern/aurea), talks to the keyboard with LCP through `lykil-device`.

```sh
cargo run --release
```

no files and no JSON: the keyboard describes itself over LCP (name, layers, keys with position, size, matrix cell and LED), so any keyboard running lykil works.

three pages:

- keymap: pick a layer, click a key, pick what it does from the palette (letters, symbols, editing, navigation, F keys, modifiers, media, keypad, layer keys). the key changes on the keyboard right away and is saved there. a pick moves on to the next key, so a row goes fast. reset keymap asks twice
- lighting: who controls the LEDs (the keyboard's own effect, or Windows Dynamic Lighting), effect, hue, saturation, brightness, speed. the keyboard on screen previews the effect
- device: key test (keys light up while pressed), layout, uptime, scans, faults, watchdog resets, last start, storage

it finds the keyboard by itself and comes back after an unplug. close VIA while it runs: every program with the raw HID interface open gets every answer.

the whole window is one canvas: `view.rs` draws it and records where every control is, `app.rs` handles the mouse against that, `device.rs` talks to the keyboard on its own thread.

needs the Aurea redraw fix (`fix(render): repaint every tile inside the clipped region`). without it, keys go missing after a click.

later: tap-hold and one-shot bindings, macros, per-key colours, scrolling for long palettes.
