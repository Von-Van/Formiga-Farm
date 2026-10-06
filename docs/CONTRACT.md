# The design contract

Formiga Desktop owns the living colony. Formiga Farm is a workshop for how its creatures look.
It is the one companion app whose main output can become part of a creature, so the boundary
between the two is drawn tighter than Hill's or Home's: Farm proposes a design, and Desktop
alone decides whether to keep it. This document describes that agreement: what a design is,
which files pass between the two apps during a session, and how Desktop decides.

The reason for the care is the same as for the other apps. A colony can represent months of
someone's time, and Farm is an optional extra. If Farm is missing, out of date, or crashes while
a creature is being reshaped, the creature should simply keep the look it had.

The contract is two crates. `formiga-forms` is the design model and how a design is drawn;
`formiga-farm-contract` is the session: its `lib.rs` is the authority, followed by
`snapshot.rs`, `replies.rs`, `accept.rs` and `projection.rs`. Example documents for every
version (the golden fixtures) are kept in `crates/formiga-farm-contract/tests/fixtures`.

Both are drafts that live beside Farm until Desktop adopts them: the design model moves into
`formiga-core`, its drawing into `formiga-art`, and the session contract into Desktop's workspace
beside `formiga-travel`. Then Farm takes them by release tag, like the other Desktop crates.

## What a design is

A `Design` is everything about how a creature looks and nothing about who it is. It has a `form`
and the `face` features every form shares (eye shape, size and spacing, pupils, brows, mouth,
cheeks: Desktop's own `FaceGenome`). The form is one of three:

| Form | What it is | Drawn by |
| --- | --- | --- |
| `original` | The look of a companion made before recipes, from its genes | `formiga-art`, unchanged |
| `companion` | One of Desktop's five companion plans, exactly as its `CreatureDesign` recipe says | `formiga-art`, unchanged |
| `sculpted` | A `Sculpt` on one of seven newer body plans | `formiga-forms`, with Formiga's own faces |

A creature that keeps its body keeps every pixel: the renderer tests compare Farm's drawing of
every creature in Desktop's sample colony with Desktop's own, clip by clip, both ways round.

A `Sculpt` is:

- a **plan**: `compact_quadruped`, `large_quadruped`, `tall_quadruped`, `upright`, `floater`,
  `crawler` or `percher`. The plan decides how it moves (walks, ambles, floats, crawls or hops),
  which slots it has, and where its anchors are;
- a **shape**: eight proportions (head, length, depth, neck, legs, arms, girth, tail), each a
  step from 0 to 10 with 5 the plan's own;
- **parts**: at most one in each slot (ears, snout, horns, tusks, mane, tail, feet, fins, back,
  wings, shell), each a catalogue identifier such as `ears.fan` or `tail.fluke`, a size from 0 to
  10, a tilt, lift and unevenness from -3 to 3, and which of the coat's colours it is drawn in;
- a **coat**: a treatment (fur, shaggy fur, smooth skin, feathers or thick skin), five colours
  (primary, secondary, underside, accent, feature), an eye colour, and how far the underside
  reaches;
- up to four **markings**, each a kind (stripes, spots, rosettes, patches, mottle, eye patches,
  mask, socks, saddle, cap, shoulders), a colour, an amount, a size and a layout number. Markings
  are laid out across the regions of the body they belong to, so they follow any shape;
- a **face layout**, one of Formiga's twelve.

Every value is a small bounded number or a known name. Nothing in a design is a pixel, a path, an
image or code. `Design::validate` refuses anything out of range or unknown, and a reader refuses
rather than repairs. `Design::revision` names a design exactly: 32 hex digits of its SHA-256.

The design model has its own version, `formiga_forms::DESIGN_VERSION` (1).

## Who owns what

| Who | Owns | Never touches |
| --- | --- | --- |
| Desktop | The colony; every creature's identity, life and canonical look; every session directory | Farm's window and data folder |
| Farm | The window, its drafts, its personal presets, its first-party presets | The colony save, ever |
| Formiga Hill and Home | Nothing here. A kept design reaches them through Desktop's ordinary projections | Farm's documents |

## A session, file by file

Desktop opens a session from its Journal, either on one creature ("Edit in Farm") or on nothing
("Create a Formiga"). It writes a fresh session directory and starts Farm as
`formiga-farm --formiga-farm <absolute session directory>`, with nothing else. From then on the
two apps talk only through files in that directory, each written whole under a temporary name
and renamed into place:

| File | Written by | What it is |
| --- | --- | --- |
| `snapshot.json` | Desktop | `FarmSnapshot`: what the session is for, the creature's look now and its revision, what Desktop can draw and keep, the owner's shared preferences |
| `ack.json` | Farm | `FarmAck`, written once: accepted, or refused as `unsupported_version { reads }`, `invalid` or `busy` |
| `proposal.json` | Farm | `FarmProposal`, written each time the owner applies a design. Each has a `serial` one higher than the last |
| `verdict.json` | Desktop | `FarmVerdict` on one serial: what Desktop decided |
| `recall.json` | Desktop | `FarmRecall`, if Desktop ends the session first |

A creature being reshaped is never handed over. It goes on living on the desktop while Farm works
on a copy of its design. Desktop decides on a proposal while Farm is open, if it watches the
directory, and otherwise when Farm exits; either way it decides on each serial once.

### Farm's side

All in `crates/formiga-farm/src/session.rs`:

1. **Arrive.** The session id is read from the directory's name, and nothing else about the path
   is trusted. The snapshot is read with a size limit, its header checked first, then shaped and
   validated, and the creature's look checked as drawable. `ack.json` is written once. A Farm
   window holds `open.lock` in its data folder, so a session that arrives while another is open
   is refused as `busy`. A second window on the same session carries on from the last serial.
2. **Work.** Every change is kept in a local draft as the owner pauses, so a crash loses
   nothing. Nothing is written to the session.
3. **Apply.** The design is written as a proposal. Farm watches for the verdict on that serial.
4. **Listen.** Once a frame, Farm looks for `recall.json`, or for `snapshot.json` having
   disappeared. Either ends the session; the draft is kept.

## The snapshot

The snapshot is everything Farm is allowed to know, and it is narrow on purpose:

- **mode**: `create { stand_in }`, the look Desktop would draw a newcomer over before it is
  anyone; or `edit_existing { creature }`, with the creature's id, name (for the window's title),
  role, its look exactly as a trip carries it (`TravelAppearance`), the sculpt Desktop keeps for
  it if any, its `revision`, its stature and scale, what it is wearing (to show on the new form),
  and whether it is `available` or `away` with another companion app.
- **capabilities**: what Desktop will do with a proposal: `edit_existing`, `create_new`,
  `sculpted_forms` (it can keep and draw sculpted forms) and `journal_note` (it notes a kept
  change in its own words). Farm proposes only what is offered; an unknown capability reads as
  `unknown`.
- **renderer**: the design model version Desktop draws, and the sculpted plans it can draw. A
  plan a newer Desktop names that this build does not know is left out as it is read.
- **presentation**: reduced motion, theme and text size.

Nothing about temperament, habits, memories, relationships, family, history, the desktop the
creature lives on, or the colony's seed. A test reads the sample snapshot for those words.

## What Desktop keeps

`accept_proposal(seal, snapshot, proposal, current)` is the whole rule. In plain terms:

- A proposal for another session or snapshot changes nothing.
- A proposal that does not validate changes nothing.
- A proposal Desktop did not offer to take, or for a design it cannot draw (a sculpted form
  without `sculpted_forms`, or on a plan it does not name), is `unsupported`.
- An edit must be for the creature the session opened on, made from the revision it has now.
  If the creature's look has changed since, the answer is `stale`, with its design now, so Farm
  can reopen it, compare against it, or keep its own as a preset. Nothing is merged.
- A creature that is away, or gone from the colony, keeps its look; the answer says which.
- Only a creature that already has its original look can keep it. A new companion needs a body.

Then `apply_design(creature, design)` makes the change, and it changes only:

- `appearance.design` and `origin.design` (the recipe, as Desktop always keeps both), and
- `appearance.face`;

and returns the sculpt Desktop keeps beside the creature, if the design has one. Its id, name,
temperament, family, habits, memories, relationships and history are not part of a design and
are never touched; a test checks every one of them, and everyone else in the colony, is
unchanged.

A sculpted creature still keeps a companion recipe: the companion plan nearest its own, in its
colours, starting from the recipe it had (`Design::fallback_recipe`). Any reader that cannot draw
sculpts, an older Hill or Home or a share code, draws that instead of nothing.

A new companion is a design and nothing else. Desktop takes it into its own welcome, which gives
the newcomer an id, a name and a life of its own, and then applies the design to it with the same
`apply_design`. Farm never invents a creature.

## The verdict

| Verdict | Meaning |
| --- | --- |
| `kept { revision }` | The creature has its new look; its revision now |
| `welcomed` | The design went into Desktop's welcome for a new companion |
| `stale { current, revision }` | The creature's look changed after the session opened |
| `unavailable` | The creature is out with another companion app; nothing changed |
| `gone` | The creature is no longer in the colony |
| `unsupported` | Desktop cannot keep this design, or did not offer what was asked |
| `invalid` | The proposal did not check out |

An unknown verdict from a newer Desktop reads as `unknown`, and Farm treats it as nothing kept.

## Versions and limits

Every document carries `format`, `version` and `min_reader_version`, read exactly as
`formiga-travel` reads its own: a reader accepts a document whose `min_reader_version` it meets,
ignores fields it does not know, and refuses anything else. Every document is size-limited before
it is parsed (64 KiB for a snapshot, 32 KiB for a proposal or verdict, 4 KiB for the rest), every
string is checked as plain text, and serials run from 1 to 10,000. The limits are in `limits` in
`lib.rs`.

| Farm version | What it added |
| --- | --- |
| 1 | Everything |

## Finding Farm

`discovery` in `lib.rs` names what Desktop looks for, following the same arrangement as Hill and
Home:

- On macOS, the bundle id `com.formiga.farm`, with `FormigaFarmVersion` in its `Info.plist`.
- On Windows, `HKCU\Software\Formiga\Farm` (or the same key under `HKLM`), with `Path`,
  `Version` and `FarmVersion` values.
- For development, `FORMIGA_FARM_PATH`.

`formiga-farm --farm-version` prints the version for packaging scripts to write.
