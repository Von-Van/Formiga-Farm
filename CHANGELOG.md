# Changelog

This page records what changed in each version of Formiga Farm. Each version's section doubles as
its release notes: `.github/workflows/release.yml` publishes it alongside the downloads when the
version is tagged.

## 0.1.0 - 2026-10-12

The first preview of Formiga Farm. It is a workshop for drawing a new Formiga, or reshaping
one from your colony, over an underground ant farm. It hands each look to Formiga Desktop, which
decides whether to keep it, and it never changes a creature's name, nature, friends or story.

- Formiga Desktop 0.67.6 opens Farm from a companion's notebook page ("Reshape in Formiga
  Farm") and from the studio ("Draw one in Formiga Farm"). Farm is built on Desktop 0.67.14's
  own design model, renderer and Farm contract, draws every creature exactly as Desktop does, and
  rehearses with any colony saved by Desktop up to 0.67.14.
- A new Formiga drawn in Farm waits in Desktop's studio to be adopted and named.
- Formiga Hill and Home can't draw sculpted bodies yet, so a creature with one visits them as its
  nearest companion body, in its colours. Farm's Home and Hill previews show it exactly that way
  and say so.
- The drafts shelf holds 64. When it is full it says so before the next new draft lets the
  oldest go, and names the draft it let go.
- A look can be shared as a design code: **Copy code** puts the look on the workbench on the
  clipboard as one line of text, and **From a design code** starts from one someone sends you. A
  code holds only a look and its name, never anyone in the colony, and Farm refuses any code it
  cannot read exactly rather than guessing.
- Twenty-one animals to start from, among them a red fox with a white muzzle, dark socks and a
  white-tipped brush, plus seven sculpted body forms and Desktop's five companion bodies.
- Farm comes as a universal Mac app, in a disk image and a zip, and for Windows as an installer and
  a portable zip, each with a checksum. Once installed, Formiga Desktop finds Farm on its own.
  A signed build notarises the app itself, so the zip opens without a warning as well as the disk
  image; until builds are signed, open it the first time with Control-click → Open.
- Opened on its own, from Applications or the Start menu, Farm says first that your own
  companions come from Formiga, and how to open one in Farm ("Reshape in Formiga Farm" on a
  companion's page, or "Draw one in Formiga Farm" in the studio), with a look around the sample
  colony or Close. `--sample` and Desktop's sessions open as before.
- Farm has its own icon: a slice of the ant farm in its wooden frame, with a Formiga sitting on
  the grass above the tunnels. The window, the Mac app and the Windows installer all wear it.
