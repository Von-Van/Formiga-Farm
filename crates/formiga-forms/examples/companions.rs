//! The sample colony as Desktop draws it, beside the sheet: `--example companions -- out.png`
mod common;
use formiga_art::{Canvas, CreatureRenderer};
use formiga_core::ActionKind;

fn main() {
    let out = std::env::args().nth(1).unwrap();
    let colony = formiga_travel::sample::colony(3);
    let scale = 4;
    let cell = 48 * scale;
    let mut canvas = Canvas::new((cell * colony.creatures.len()) as u32, (cell * 2) as u32);
    for (i, c) in colony.creatures.iter().enumerate() {
        for (row, action) in [ActionKind::Idle, ActionKind::Traverse]
            .into_iter()
            .enumerate()
        {
            let f = CreatureRenderer::render_studio_frame(&c.appearance, action, 0);
            common::blit(
                &mut canvas,
                &f,
                i * cell,
                row * cell,
                scale,
                (i + row) % 2 == 0,
            );
        }
    }
    common::write(&canvas, &out);
}
