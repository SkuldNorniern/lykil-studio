# lykil-studio

Lykil Studio: manage lykil keyboards. built on [Aurea](https://github.com/SkuldNorniern/aurea), talks to the keyboard through `lykil-device`.

```sh
cargo run --release
```

on Linux, hidapi needs `libudev-dev` (`sudo apt install libudev-dev`), and a normal user needs a udev rule to open the keyboard's hidraw node. the Windows tab only exists on Windows. macOS builds but stalls right after the window opens, not solved yet.

a lykil keyboard needs no files: it describes itself over LCP (name, layers, keys with position, size, matrix cell and LED). a keyboard that only speaks VIA (stock QMK) works too, see below. when a keyboard speaks both, Studio uses LCP.

pages (Ctrl+1 to Ctrl+5, Ctrl+Tab, or the mouse wheel over the tabs):

- keymap: pick a layer, click a key, pick what it does from the palette. it changes on the keyboard right away and is saved there. a pick moves on to the next key. "when held" makes a key dual-role, "send with" adds modifiers. arrows move the selection, Delete clears a key
- macros: 16 macros, type the text one should type (US layout, up to 32 steps), save, then bind it from the keymap palette
- lighting: effect cards with a small live preview each, one colour / two colours / rainbow, a colour picker (square, hue bar, hex and RGB fields, Ctrl+C / Ctrl+V), brightness, speed, background and size, lighting the keys of a held layer, and who controls the LEDs. the keyboard on screen runs the firmware's own effect code, so it shows what the LEDs do. click keys there to try the press effects. per-key: paint with the brush, right click takes a key's colour
- device: key test, layout, firmware and version, uptime, counters, storage
- windows: whether Windows Dynamic Lighting may take the LEDs, and a way to its settings

## VIA only keyboards

VIA keyboards do not describe themselves, so Studio needs the keyboard's VIA definition: the JSON VIA's Design tab loads. put it in the `via` folder of Studio's data folder: `%APPDATA%\Lykil Studio` on Windows, `~/Library/Application Support/Lykil Studio` on macOS, `~/.config/Lykil Studio` on Linux (the page shows the folder and a button to open it). Studio matches it by USB vendor and product id, else by vendor id and name (so a wireless dongle, `EVO80 2.4G`, takes the `EVO80` definition), and picks it up by itself.

then the keymap page works as usual, over VIA: keycodes are read and written as QMK keycodes and translated with `lykil-qmk`. palette entries VIA has no keycode for are dimmed, and a group with none is marked Lykil only.

- macros: the keyboard's dynamic macros, in QMK's `send_string` bytes. same editor as for lykil keyboards; the limit is the keyboard's macro buffer
- lighting: the settings the definition's `menus` offer (VIA's built-in `qmk_rgb_matrix`, `qmk_rgblight` and the rest, or the keyboard's own), one card per section: sliders, effect list, colour, on/off. each channel is saved to the keyboard's EEPROM once you let go. `showIf` is not read yet, so every setting shows

`lykil device via-json` writes such a definition for a lykil keyboard, for VIA itself. `LYKIL_STUDIO_VIA=1` makes Studio talk VIA to a keyboard that speaks LCP too, to try this path.

## notes

english and korean: follows the system language, `LYKIL_LANG=ko` or `en` overrides, and the button in the header switches. text is looked up in `lang.rs`; anything missing stays english. korean uses Malgun Gothic (Apple SD Gothic Neo on macOS, Noto Sans CJK on Linux), since the renderer takes one font per text.

close VIA while Studio runs: every program with the raw HID interface open gets every answer.

the whole window is one canvas. how it is split:

- `view.rs`: the frame (header, tabs, footer) and which page to draw. it records where every control is
- `pages/`: one file per page. `widgets/`: controls more than one page uses (sliders, chips, segmented bars, the colour square and hue bar). `keyboard.rs`: any keyed device drawn from its description
- `app/`: state and input, matched against the recorded controls, one file per page
- `devices/`: the connected device on its own thread. `Keyboard` is what Studio knows, `Command` what it asks; `lcp.rs` talks to Lykil keyboards, `via.rs` to VIA ones. another kind of device is another driver next to these
- `lamps/`: other Dynamic Lighting devices on Windows and where they sit on the desk
- `anim.rs` eases hover, tab and page changes

Aurea bugs Studio works around are in `../aurea_bug.md`.
