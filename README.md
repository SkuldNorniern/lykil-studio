# lykil-studio

Lykil Studio: manage lykil keyboards. built on [Aurea](https://github.com/SkuldNorniern/aurea), talks to the keyboard with LCP through `lykil-device`.

```sh
cargo run --release
```

no files and no JSON: the keyboard describes itself over LCP (name, layers, keys with position, size, matrix cell and LED), so any keyboard running lykil works.

four pages:

- keymap: pick a layer, click a key, pick what it does from the palette (letters, symbols, editing, navigation, F keys, modifiers, media, keypad, layer keys, one-shot, macros). the key changes on the keyboard right away and is saved there. a pick moves on to the next key, so a row goes fast. "when held" makes a key dual-role (tap the key, hold for a modifier or a layer), "send with" adds modifiers (Shift+1). reset keymap asks twice and keeps the macros
- macros: 16 macros, type the text one should type (US layout, up to 32 steps), save, then bind it from the keymap palette
- lighting: who controls the LEDs (the keyboard's own effect, or Windows Dynamic Lighting), effect, hue, saturation, brightness, speed. the keyboard on screen previews the effect. with "per-key", click or drag over keys to paint them in the hue and saturation picked
- device: key test (keys light up while pressed), layout, uptime, scans, faults, watchdog resets, last start, storage

it finds the keyboard by itself and comes back after an unplug. close VIA while it runs: every program with the raw HID interface open gets every answer.

the whole window is one canvas: `view.rs` draws it and records where every control is, `app.rs` handles the mouse against that, `device.rs` talks to the keyboard on its own thread.

needs the Aurea redraw fix (`fix(render): repaint every tile inside the clipped region`). without it, keys go missing after a click.

later: macros with delays and held keys from the UI (the keyboard already plays them), VIA's macro tab, scrolling for long palettes.
