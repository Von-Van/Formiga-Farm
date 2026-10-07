//! A review sheet of every starter in every intent: `cargo run -p formiga-forms --example sheet -- out.png`
use formiga_art::Canvas;
use formiga_forms::{Design, DesignRenderer, Form, Intent, Plan, Sculpt, plain_face};

mod common;
use common::{blit, write};

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "sheet.png".into());
    let base = formiga_travel::sample::colony(3).creatures[0]
        .appearance
        .clone();
    let scale = 4;
    let frames = 6;
    let cell = 48 * scale;
    let rows = Plan::ALL.len();
    let cols = Intent::ALL.len() * 2;
    let mut canvas = Canvas::new((cols * cell) as u32, (rows * cell) as u32);
    for (row, plan) in Plan::ALL.into_iter().enumerate() {
        let design = Design {
            form: Form::Sculpted {
                sculpt: Sculpt::starter(plan),
            },
            face: plain_face(),
        };
        for (i, intent) in Intent::ALL.into_iter().enumerate() {
            let (count, _) = DesignRenderer::timing(intent);
            for (j, frame) in [0u8, (count / 2).min(frames)].into_iter().enumerate() {
                let f = DesignRenderer::intent_frame(&design, &base, intent, frame, true, false);
                blit(
                    &mut canvas,
                    &f,
                    (i * 2 + j) * cell,
                    row * cell,
                    scale,
                    (i + row) % 2 == 0,
                );
            }
        }
    }
    write(&canvas, &out);
}
