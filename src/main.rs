#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod app;
#[cfg(target_os = "linux")]
mod audio;
#[cfg(target_os = "linux")]
mod device;
#[cfg(target_os = "linux")]
mod drm_display;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod minigame;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod notifications;
mod render;
mod state;

use app::App;
use minigame::{GameInput, GameKind, MiniGame};
#[cfg(target_os = "linux")]
use notifications::{WebhookConfig, WebhookServer};
use state::{Action, PetState};
use std::{
    error::Error,
    path::{Path, PathBuf},
};

fn argument(args: &[String], name: &str) -> Result<Option<String>, Box<dyn Error>> {
    match args.iter().position(|arg| arg == name) {
        Some(i) => Ok(Some(
            args.get(i + 1)
                .filter(|v| !v.starts_with("--"))
                .ok_or_else(|| format!("{name} needs a value"))?
                .clone(),
        )),
        None => Ok(None),
    }
}

fn action(name: &str) -> Result<Action, Box<dyn Error>> {
    Ok(match name {
        "idle" => Action::Idle,
        "pet" => Action::Pet,
        "feed" => Action::Feed,
        "play" => Action::Play,
        "sleep" => Action::Sleep,
        _ => return Err(format!("unknown action: {name}").into()),
    })
}

fn load_state(path: &Path) -> Result<PetState, Box<dyn Error>> {
    match PetState::load(path) {
        Ok(state) => Ok(state),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(PetState::default()),
        Err(err) => Err(format!("cannot load {} (file preserved): {err}", path.display()).into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|v| v == "--help") {
        println!("muku-pet --device [--state FILE] [--seconds N] [--snapshot FILE]\n\
                         [--webhook-config FILE]\n\
                  muku-pet --preview FILE.png [--action idle|pet|feed|play|sleep] [--time SECONDS]\n\
                         [--notice-title TEXT --notice-body TEXT --notice-source SOURCE]\n\
                  muku-pet --preview-game star|beat|select|result FILE.png [--game-time SECONDS]\n\
                  muku-pet --simulate pet,feed,play,sleep --state FILE\n\
                  muku-pet --inspect\n\
                  Device: A/B/Y give care, X opens two mini-games; START exits from home.\n\
                  No wall clock or power-off penalties. Network is opt-in; firmware is untouched.");
        return Ok(());
    }
    let state_path =
        PathBuf::from(argument(&args, "--state")?.unwrap_or_else(|| "/tmp/muku-state.json".into()));
    let mut state = load_state(&state_path)?;
    if let Some(actions) = argument(&args, "--simulate")? {
        for name in actions.split(',') {
            state.act(action(name)?);
            state.tick(1.0);
        }
        state.save(&state_path)?;
        println!("{}", serde_json::to_string_pretty(&state)?);
        return Ok(());
    }
    if let Some(path) = argument(&args, "--preview")? {
        if let Some(name) = argument(&args, "--action")? {
            state.act(action(&name)?);
            state.tick(0.65);
        }
        let t: f32 = argument(&args, "--time")?
            .unwrap_or_else(|| "1.25".into())
            .parse()?;
        if !t.is_finite() || t < 0.0 {
            return Err("time must be finite and nonnegative".into());
        }
        let mut app = App::new(state);
        if let Some(title) = argument(&args, "--notice-title")? {
            app.push_notice(notifications::Notice {
                id: "preview".into(),
                title,
                body: argument(&args, "--notice-body")?.unwrap_or_default(),
                source: argument(&args, "--notice-source")?.unwrap_or_else(|| "ai".into()),
                kind: "notice".into(),
                audio_wav_base64: None,
            });
        }
        render::render_app(&app, t).save_png(&path)?;
        println!("preview: {path}");
        return Ok(());
    }
    if let Some(view) = argument(&args, "--preview-game")? {
        let path = args
            .get(
                args.iter()
                    .position(|arg| arg == "--preview-game")
                    .expect("preview-game exists")
                    + 2,
            )
            .ok_or("--preview-game needs an output path")?;
        let mut app = App::new(state.clone());
        app.screen = app::Screen::GameSelect;
        let game_time: f32 = argument(&args, "--game-time")?
            .unwrap_or_else(|| "4.0".into())
            .parse()?;
        if !game_time.is_finite() || game_time < 0.0 {
            return Err("game time must be finite and nonnegative".into());
        }
        match view.as_str() {
            "select" => {}
            "star" | "beat" => {
                let kind = if view == "star" {
                    GameKind::StarCatch
                } else {
                    GameKind::BeatGarden
                };
                app.screen = app::Screen::Playing;
                let mut game = MiniGame::new(kind, 7);
                for _ in 0..(game_time / 0.1).round() as usize {
                    game.tick(0.1);
                    if kind == GameKind::StarCatch && game_time > 2.0 {
                        game.input(GameInput::Right);
                    }
                }
                app.game = Some(game);
            }
            "result" => {
                app.screen = app::Screen::Result;
                app.result = Some(minigame::GameResult {
                    kind: GameKind::StarCatch,
                    score: 18,
                    best_combo: 7,
                    hits: 18,
                    misses: 2,
                });
            }
            _ => return Err(format!("unknown game preview: {view}").into()),
        }
        render::render_app(&app, game_time).save_png(path)?;
        println!("preview: {path}");
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        if args.iter().any(|v| v == "--inspect") {
            return device::inspect();
        }
        if args.iter().any(|v| v == "--device") {
            let seconds: Option<f64> = argument(&args, "--seconds")?
                .map(|v| v.parse())
                .transpose()?;
            if seconds.is_some_and(|v| !v.is_finite() || v <= 0.0) {
                return Err("seconds must be finite and positive".into());
            }
            let snapshot = argument(&args, "--snapshot")?.map(PathBuf::from);
            let webhook = if let Some(path) = argument(&args, "--webhook-config")? {
                let config = WebhookConfig::load(Path::new(&path))?;
                let server = WebhookServer::start(&config.bind, &config.token)?;
                eprintln!("webhook listening on http://{}", server.address());
                Some(server)
            } else {
                None
            };
            let mut app = App::new(state);
            return device::run(
                &mut app,
                &state_path,
                seconds,
                snapshot.as_deref(),
                webhook.as_ref(),
            );
        }
    }
    Err("use --preview on macOS, or --device / --inspect on the Linux handheld".into())
}
