# Formiga Farm

A separate desktop app where Formiga Desktop's creatures are made and remade: a creature editor
over an ant farm, opened from Desktop's Journal on one companion to reshape, or on nothing at
all to draw a new one. Read `docs/DESIGN.md` for the product, where the checkout has it, and
[docs/CONTRACT.md](docs/CONTRACT.md) for the contract with Desktop before changing either side of
it.

- Design documents stay out of git and off GitHub: `docs/DESIGN.md` and
  `docs/DESKTOP_ADOPTION.md` are kept locally and ignored. Keep them current, but never add
  them, or any other design or planning document, to a commit.

## Relationship to Formiga Desktop

- Desktop lives at `../Formiga Desktop` (GitHub `Von-Van/Formiga-Desktop`, public). `formiga-core`,
  `formiga-art` and `formiga-travel` come from Desktop by one release tag in the root
  `Cargo.toml`, so their types agree. Never copy their source into this repository.
- Two crates here are drafts of things that belong to Desktop, kept here until it adopts them,
  as Formiga Home keeps its contract:
  - `formiga-forms`: the design model (`Design`, `Form`, `Sculpt`) that belongs in
    `formiga-core`, and the drawing of sculpted forms (`DesignRenderer`) that belongs in
    `formiga-art`.
  - `formiga-farm-contract`: the session contract, which moves into Desktop's workspace beside
    `formiga-travel`.
  Change them as Desktop's own crates would be changed: versioned, with golden fixtures
  untouched.
- Desktop is authoritative for the colony. Farm never reads or writes Desktop's `colony.json` at
  runtime. The only exception is `--from-save`, which reads (never writes) a save and projects it
  as Desktop would.
- Do not edit the Desktop checkout from a Farm session. Propose Desktop-side changes instead, in
  `docs/DESKTOP_ADOPTION.md`.

## The design contract

- Farm proposes; Desktop decides. A proposal is a design and what it is for, nothing else: no
  ids claimed, no names, no temperament, family, habits, relationships, history or prose.
  Desktop runs `accept_proposal` on every one and changes a creature only through
  `apply_design`, which touches its look and nothing else.
- An edit names the revision of the design it was made from; a creature that has changed since
  is answered `Stale` with its design now. Nothing is merged silently.
- A new companion is a design handed to Desktop's own welcome, which gives it an id, a name and
  a life. Farm never invents a creature.
- A creature is never handed over: it lives on the desktop the whole time Farm is open. Whatever
  goes wrong (Farm missing, refusing, crashing, writing nonsense) the creature keeps its look.
- Rehearsals (`--sample`, `--sample-edit`, `--from-save`) stand in for Desktop through the
  contract's own `accept_proposal` and `apply_design`, so a rehearsal proves the same loop a real
  session does.

## Design rules that apply to every change

- A creature that keeps its body draws exactly as Desktop draws it: companion recipes and the
  original look go through `formiga-art` untouched, and `tests/render.rs` holds that to the
  pixel. Never redraw them here.
- A sculpted form wears Formiga's own faces (`render_face_frame`), so it keeps every expression,
  and answers every clip Desktop bakes, inside its 48-pixel frame with a pixel to spare.
- Body plans move as themselves: a floater hovers and never walks on legs it does not have, a
  percher hops, a crawler paddles. Every plan maps Desktop's clips to the same few intents.
- A design is semantic: named, bounded steps and catalogue identifiers, never pixels, paths,
  images or code. A design from anywhere else is validated and refused, never repaired.
- Presets are starting points only: no traits, rarity, stats or advantages, and nothing about a
  creature remembers which one it began as except a lineage note nobody acts on.
- Soft play: no currency, timers, unlocks or pressure. The ant farm is scenery; it is slower and
  lower in contrast than the stage and never stands in for a control.
- Creature art follows Formiga's rules: pixel scale, light from the upper left, the shared
  outline ink, colours softened as Desktop softens a recipe's.
- Reduced motion gets held poses and cuts, not slower animation.

## Conventions

- Rust 1.97.1, edition 2024, matching Desktop. Match Desktop's code style: doc comments that say
  why, plain names, tests named as sentences (`a_floater_hovers_and_never_walks_on_legs_it_does_not_have`).
- The gate, which CI runs on macOS and Windows:
  `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
- `--render-presets <png>`, `--render-poses <preset> <png>` and `--render-habitat <png>` draw
  without a window. Look at them, cropped and enlarged, after changing anything visual. For the
  window itself, `--snap <png>` (with `--layer`, `--intent`, `--view`, `--shelf`, `--theme`,
  `--preset` and a scratch `FORMIGA_FARM_DATA_DIR`) pictures it and closes; never capture the
  screen instead.
- `FORMIGA_FARM_BLESS=1 cargo test -p formiga-farm-contract --test golden` writes only the current
  version's fixtures, and only when a new version is being made.
- Never commit planning material: checklists, roadmaps, "next" lists. Docs describe what exists.
