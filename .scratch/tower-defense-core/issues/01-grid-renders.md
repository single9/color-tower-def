# 01: Grid renders

**What to build:** A Bevy app boots into a 700x500 window showing the 25x25 Grid as 20px color-block Cell, with the Spawn Cell and Goal Cell visually distinct from Buildable Cell using the neutral palette (Buildable = light gray, Spawn = dark purple, Goal = orange). The right 200px is a static sidebar shell (no live data or interactivity yet — placeholder labels for Gold/Lives/Wave and placeholder Tower Kind buttons are fine).

**Blocked by:** None (can start immediately)

**Status:** done

- [x] Running the app opens a 700x500 window
- [x] The left 500x500 area renders a 25x25 grid of 20px Cell
- [x] Exactly one Cell is rendered as Spawn (dark purple) and one as Goal (orange); all other Cell render as Buildable (light gray)
- [x] The right 200px area renders a visually separated sidebar panel

## Verification

Confirmed visually via a self-screenshot taken from inside the running Bevy app (avoids any risk of capturing unrelated desktop content).
