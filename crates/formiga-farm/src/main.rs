//! Formiga Farm: the workshop where Formigas are made and remade. It opens from Desktop's
//! Journal, on a creature to reshape or on nothing at all, and hands Desktop a design to keep.

mod app;
mod editor;
mod habitat;
mod icon;
mod import;
mod notices;
mod paint;
mod presets;
mod review;
mod session;
mod store;

use anyhow::{Context, Result, bail};
use app::{FarmApp, Opening, Shelf, View};
use editor::Layer;
use formiga_forms::Intent;
use session::{Host, Rehearsal};
use std::path::PathBuf;

const USAGE: &str = "\
Usage: formiga-farm [--sample | --sample-edit <N> | --formiga-farm <SESSION DIRECTORY>
                     | --from-save <FILE> [--creature <N>]]

  --sample                 Rehearse drawing a new Formiga for Desktop's sample colony (the default)
  --sample-edit <N>        Rehearse reshaping creature N of Desktop's sample colony
  --formiga-farm <DIR>     How Desktop opens Farm: the session it wrote
  --from-save <FILE>       Development only: rehearse with a Desktop colony file, which is only
                           ever read; with --creature <N>, reshape its creature N
  --preset <ID>            Open on a preset: animal.giant_panda, form.floater, ...

For review, without a window:
  --render-presets <PNG>   Every preset at rest and walking
  --render-poses <ID> <PNG>  One preset in every pose, every frame
      Add --detail <1-4> to either to draw at that many pixels to each frame pixel
      (Farm's own is 2), or --pixels for Desktop's 48-pixel frames
  --render-habitat <PNG>   The ant farm behind the workbench
  --import <IMAGE> <PNG>   Read a picture into designs, and draw the picture beside them

For review, of the window itself:
  --snap <PNG>             Open the window, and after --at seconds (3 if not given) save a
                           picture of it and close
  --layer <form|parts|finish|face>  --intent <idle|move|...>  --view <stage|desktop|home|hill>
  --shelf <animals|forms|mine|drafts>  --theme <light|dark>
  --picture <IMAGE>        Open reading a picture, as From a picture does

  --farm-version           Print the newest Farm version this build reads, for packaging
  --icon <FOLDER>          Write Farm's icon as .icns, .ico and .png, for packaging
";

enum Source {
    Sample,
    SampleEdit(usize),
    Session(PathBuf),
    Save(PathBuf, Option<usize>),
}

struct Args {
    source: Source,
    preset: Option<String>,
    snap: Option<PathBuf>,
    picture: Option<PathBuf>,
    at: f64,
    layer: Option<Layer>,
    intent: Option<Intent>,
    view: Option<View>,
    shelf: Option<Shelf>,
    theme: Option<String>,
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Option<Args>> {
    let mut parsed = Args {
        source: Source::Sample,
        preset: None,
        snap: None,
        picture: None,
        at: 3.0,
        layer: None,
        intent: None,
        view: None,
        shelf: None,
        theme: None,
    };
    let mut creature = None;
    let mut save = None;
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| args.next().with_context(|| format!("{flag} needs a value"));
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--sample" => parsed.source = Source::Sample,
            "--sample-edit" => {
                parsed.source = Source::SampleEdit(
                    value("--sample-edit")?
                        .parse()
                        .context("--sample-edit is a number")?,
                )
            }
            formiga_farm_contract::LAUNCH_ARGUMENT => {
                parsed.source = Source::Session(PathBuf::from(value(
                    formiga_farm_contract::LAUNCH_ARGUMENT,
                )?))
            }
            "--from-save" => save = Some(PathBuf::from(value("--from-save")?)),
            "--creature" => {
                creature = Some(
                    value("--creature")?
                        .parse()
                        .context("--creature is a number")?,
                )
            }
            "--preset" => parsed.preset = Some(value("--preset")?),
            "--snap" => parsed.snap = Some(PathBuf::from(value("--snap")?)),
            "--picture" => parsed.picture = Some(PathBuf::from(value("--picture")?)),
            "--at" => {
                parsed.at = value("--at")?
                    .parse()
                    .context("--at is a number of seconds")?
            }
            "--layer" => {
                parsed.layer = Some(match value("--layer")?.as_str() {
                    "form" => Layer::Form,
                    "parts" => Layer::Parts,
                    "finish" => Layer::Finish,
                    "face" => Layer::Face,
                    other => bail!("no layer {other}"),
                })
            }
            "--intent" => {
                let name = value("--intent")?;
                parsed.intent = Some(
                    Intent::ALL
                        .into_iter()
                        .find(|intent| intent.label().eq_ignore_ascii_case(&name))
                        .with_context(|| format!("no intent {name}"))?,
                );
            }
            "--view" => {
                parsed.view = Some(match value("--view")?.as_str() {
                    "stage" => View::Stage,
                    "desktop" => View::Desktop,
                    "home" => View::Home,
                    "hill" => View::Hill,
                    other => bail!("no view {other}"),
                })
            }
            "--shelf" => {
                parsed.shelf = Some(match value("--shelf")?.as_str() {
                    "animals" => Shelf::Animals,
                    "forms" => Shelf::BodyForms,
                    "mine" => Shelf::Mine,
                    "drafts" => Shelf::Drafts,
                    other => bail!("no shelf {other}"),
                })
            }
            "--theme" => parsed.theme = Some(value("--theme")?),
            other => bail!("unexpected argument {other}\n\n{USAGE}"),
        }
    }
    if let Some(save) = save {
        parsed.source = Source::Save(save, creature);
    }
    Ok(Some(parsed))
}

/// The detail a review picture is drawn at: Farm's own unless `--detail` says, or 1 with
/// `--pixels`.
fn review_detail(raw: &[String]) -> Result<u32> {
    if raw.iter().any(|arg| arg == "--pixels") {
        return Ok(1);
    }
    match raw.iter().position(|arg| arg == "--detail") {
        Some(at) => {
            let detail: u32 = raw
                .get(at + 1)
                .and_then(|value| value.parse().ok())
                .context("--detail is a number from 1 to 4")?;
            anyhow::ensure!(
                (1..=formiga_forms::MAX_DETAIL).contains(&detail),
                "--detail is a number from 1 to 4"
            );
            Ok(detail)
        }
        None => Ok(formiga_forms::DETAIL),
    }
}

/// How many picture pixels a frame pixel takes in a review sheet: a whole number of each
/// drawn pixel, about four.
fn review_scale(detail: u32) -> i32 {
    match detail {
        1 => 3,
        detail => (detail * (4 / detail).max(1)) as i32,
    }
}

fn main() -> Result<()> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    match raw.first().map(String::as_str) {
        Some("--farm-version") => {
            println!("{}", formiga_farm_contract::FARM_FORMAT_VERSION);
            return Ok(());
        }
        // For the packaging scripts too: Farm's icon, for the bundle and the installer.
        Some("--icon") => {
            let folder = PathBuf::from(
                raw.get(1)
                    .context("--icon needs a folder to write the icon into")?,
            );
            std::fs::create_dir_all(&folder)?;
            std::fs::write(folder.join("FormigaFarm.icns"), icon::icns())?;
            std::fs::write(folder.join("FormigaFarm.ico"), icon::ico())?;
            std::fs::write(folder.join("FormigaFarm.png"), icon::png(&icon::at(1024)))?;
            println!("Wrote Farm's icon to {}", folder.display());
            return Ok(());
        }
        Some("--render-presets") => {
            let path = raw.get(1).context("--render-presets needs a file")?;
            let detail = review_detail(&raw)?;
            review::write_png(
                &review::presets_sheet(review_scale(detail), detail),
                path.as_ref(),
            )?;
            println!("Drew every preset to {path}");
            return Ok(());
        }
        Some("--import") => {
            let image = raw.get(1).context("--import needs a picture")?;
            let path = raw.get(2).context("--import needs a file to draw to")?;
            let started = std::time::Instant::now();
            let picture = import::open(image.as_ref())?;
            let takes = import::read_image(&import::small(&picture), &review::stand_in())?;
            let seconds = started.elapsed().as_secs_f32();
            let picture = picture.thumbnail(256, 256).to_rgba8();
            review::write_png(&review::import_sheet(&picture, &takes), path.as_ref())?;
            for take in &takes {
                println!("{} ({}%): {}", take.title, take.likeness, take.summary);
            }
            println!("Read {image} in {seconds:.1}s and drew it to {path}");
            return Ok(());
        }
        Some("--render-poses") => {
            let id = raw.get(1).context("--render-poses needs a preset")?;
            let path = raw.get(2).context("--render-poses needs a file")?;
            let preset = presets::find(id).with_context(|| format!("no preset {id}"))?;
            let detail = review_detail(&raw)?;
            review::write_png(
                &review::poses_sheet(&preset.design, review_scale(detail), detail),
                path.as_ref(),
            )?;
            println!("Drew {} in every pose to {path}", preset.name);
            return Ok(());
        }
        Some("--render-habitat") => {
            let path = raw.get(1).context("--render-habitat needs a file")?;
            let habitat = habitat::Habitat::new(400, 260, 11, habitat::Daylight::Day);
            review::write_png(&review::scaled(&habitat.canvas, 3), path.as_ref())?;
            println!("Drew the habitat to {path}");
            return Ok(());
        }
        _ => {}
    }
    let Some(args) = parse_args(raw.into_iter())? else {
        print!("{USAGE}");
        return Ok(());
    };
    let data = store::folder();
    let open = data.as_deref().map(store::take);
    let busy = matches!(open, Some(Err(store::Busy)));
    let (host, snapshot) = match args.source {
        Source::Session(dir) => {
            let (visit, snapshot) = session::arrive(&dir, busy)?;
            (Host::Desktop(visit), snapshot)
        }
        source => {
            if busy {
                bail!("Formiga Farm is already open");
            }
            let (colony, index, label) = match source {
                Source::SampleEdit(index) => (
                    session::colony_from(None)?,
                    Some(index),
                    "Rehearsing with Desktop's sample colony".to_owned(),
                ),
                Source::Save(path, index) => (
                    session::colony_from(Some(&path))?,
                    index,
                    "Rehearsing with a colony file, read only".to_owned(),
                ),
                _ => (
                    session::colony_from(None)?,
                    None,
                    "Rehearsing with Desktop's sample colony".to_owned(),
                ),
            };
            let (rehearsal, snapshot) = Rehearsal::open(colony, index, label)?;
            (Host::Rehearsal(Box::new(rehearsal)), snapshot)
        }
    };
    let start = match &args.preset {
        Some(id) => {
            let preset = presets::find(id).with_context(|| format!("no preset {id}"))?;
            Some((
                preset.design,
                Some(preset.id.to_owned()),
                preset.name.to_owned(),
            ))
        }
        None => None,
    };
    let title = match snapshot.creature() {
        Some(creature) => format!("Formiga Farm \u{2014} {}", creature.name),
        None => "Formiga Farm \u{2014} a new Formiga".to_owned(),
    };
    let store = store::Store::new(data.clone());
    let place = data.as_deref().and_then(store::WindowPlace::load);
    let text_scale = app::style::text_scale(&snapshot.presentation);
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(place.map_or([1240.0 * text_scale.min(1.2), 800.0], |p| {
            [p.width, p.height]
        }))
        .with_min_inner_size([620.0, 520.0])
        .with_icon(icon::window());
    if let Some(place) = place {
        viewport = viewport.with_position([place.x, place.y]);
    }
    if args.snap.is_some() {
        viewport = viewport.with_active(false).with_inner_size([1240.0, 800.0]);
    }
    let opening = Opening {
        snapshot,
        host,
        store,
        open: open.and_then(Result::ok),
        start,
    };
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Formiga Farm",
        options,
        Box::new(move |cc| {
            let mut app = FarmApp::new(&cc.egui_ctx, opening).map_err(|error| error.to_string())?;
            app.show(args.layer, args.intent, args.view);
            if let Some(shelf) = args.shelf {
                app.shelf(shelf);
            }
            match args.theme.as_deref() {
                Some("dark") => cc.egui_ctx.set_theme(eframe::egui::ThemePreference::Dark),
                Some("light") => cc.egui_ctx.set_theme(eframe::egui::ThemePreference::Light),
                _ => {}
            }
            if let Some(path) = args.picture {
                app.read_picture(path);
            }
            if let Some(path) = args.snap {
                app.snap(path, args.at);
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|error| anyhow::anyhow!("the window could not open: {error}"))
}
