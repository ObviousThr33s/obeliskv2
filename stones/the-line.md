# the line

```text
┌─ a → b ─────────────────────────────┐
│                                     │
│ a key                               │
│ handle_events reads it              │
│ it returns a move                   │
│ run() takes the move                │
│ field.move_entity shifts the player │
│ the field holds the new position    │
│ redraw is true                      │
│ state becomes Render                │
│ render() reads the terminal size    │
│ it makes a viewport                 │
│ it reads the player's position      │
│ it gathers the others as walls      │
│ render_raycasted casts              │
│ it returns a view                   │
│ gfx::render takes the view          │
│ it writes to the terminal           │
│ b                                   │
│                                     │
└─────────────────────────────────────┘
```

a → b — the live line, slowly.
(the raycaster path. poly → light is not on it yet.)
