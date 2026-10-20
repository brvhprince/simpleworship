# NDI experiment (throwaway branch)

Can SimpleWorship take frames of its own stream page, with transparency,
while the window is hidden? This branch measures it. It is not part of the
app and will not be merged; see `apps/desktop/src-tauri/src/ndi_spike.rs`.

## Running it on Windows

Needs the usual development setup (Rust, Node 22, pnpm; see the README).

```powershell
git fetch
git switch ndi-experiment
pnpm install
$env:SW_NDI_SPIKE="C:\ndi-test"; pnpm dev
```

SimpleWorship opens as usual. After a few seconds it puts a test lower third
live and opens and closes a small window four times (on screen, hidden,
off-screen, behind the operator window). **Leave it alone for about a
minute**, until the terminal says `NDI SPIKE: done`.

Then send back:

1. Every line in the terminal containing `NDI SPIKE`.
2. `C:\ndi-test\hidden-changed.png` and `C:\ndi-test\visible.png`.

## Results on macOS (MacBook Pro, 1920×1080)

| Window                     | Transparency | One frame | New slide seen after |
| -------------------------- | ------------ | --------- | -------------------- |
| On screen                  | yes          | ~70 ms    | ~120 ms              |
| Hidden (never shown)       | yes          | ~70 ms    | ~125 ms              |
| Off-screen                 | yes          | ~70 ms    | ~130 ms              |
| Behind the operator window | yes          | ~70 ms    | ~105 ms              |

Plus ~65 ms to turn a frame into pixels. Capturing the window from outside
(as OBS does) failed for hidden and off-screen windows and needs the Screen
Recording permission.
