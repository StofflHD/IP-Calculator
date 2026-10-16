# IP Calculator (Rust, egui)

IPv4/IPv6 subnet calculator in the style of the Bitcricket IP Calculator, shipped as a single AppImage.

| Tab | Contents |
|---|---|
| Conversions | Dotted decimal, decimal, hex, binary (every field is editable), Ethernet multicast MAC |
| Classes | Class A–E, class address range, coloured bit usage (n/h), classful network |
| Subnets | Mask, subnet/host bits, max subnets/hosts (linked selection), bit usage n/s/h, subnet table |
| CIDR | Address block mask, CIDR bits, max routes, CIDR mask, bit usage c/x, route table |
| IPv6 | Address type tree (collapsible), format bar with decoded fields (Teredo, 6to4, ISATAP …), expanded form, reverse DNS |
| Info | Special-purpose blocks (RFC 1918, 6598, 5737 …), IPv4-mapped address, 6to4 prefix, reverse DNS |

- **Options**: *Allow 1 Subnet Bit (RFC 1878)*, *Allow 1 Host Bit (RFC 3021)*, Dark Theme
  (the "Rust" scheme of notepad-rs: warm neutrals with an orange accent).
  Without RFC 1878, subnet zero and the all-ones subnet are excluded (RFC 950).
- Address entry with history (▾). Input such as `10.20.0.0/22` or `10.20.0.0/255.255.252.0`
  also sets the mask (the block mask on the CIDR tab).
- Clicking a table row selects its address, right-click copies it. *File* copies the
  subnet or route list as CSV to the clipboard.
- Settings, history and the last addresses are stored in `~/.local/share/ip-calculator/`.
- **Updates** (*Help → Check for Updates*): checks the GitHub releases, downloads the new
  AppImage, verifies its SHA-256 checksum (`.sha256` asset of the release) and replaces the running
  AppImage; *Restart Now* starts the new version. By default this is checked once a day at start-up
  (*Help → Check for Updates at Start-up*, or `IPC_NO_UPDATE=1`). HTTP goes through `curl` or
  `wget`; a proxy can be set under *Help → Updater Proxy*. Same updater as notepad-rs.

## Building

```bash
cargo test                          # test the calculation core
cargo run --release                 # run directly
packaging/build-appimage.sh         # -> dist/IP-Calculator-<version>-x86_64.AppImage (+ .sha256)
```

The script downloads `appimagetool` when needed (or set `APPIMAGETOOL=/path/to/tool`).

The binary depends only on the glibc of the build system (here 2.39 → Debian 13, Ubuntu 24.04
and newer). For older distributions, build on an older system or in an older container.

## Layout

- `src/ipv4.rs` – conversions, classes, subnetting, CIDR, special-purpose registry (with tests)
- `src/ipv6.rs` – type classification (longest prefix match) and field layout (with tests)
- `src/app.rs` – user interface (eframe/egui 0.29)
- `src/update.rs` – updater (GitHub releases, SHA-256 check, replaces the AppImage; with tests)
- `src/theme.rs` – UI colour scheme "Rust" (dark and light), shared with
  [notepad-rs](https://github.com/StofflHD/notepad-rs)
- `packaging/` – desktop file, icon (`make_icon.py`, needs pycairo), AppRun, build script

## License

GPL-3.0-or-later, see [LICENSE](LICENSE). The licence is also shown under *Help → About*.

---

© StofflHD and Claude
