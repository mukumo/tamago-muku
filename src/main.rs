#[cfg(target_os = "linux")]
mod device;
#[cfg(target_os = "linux")]
mod drm_display;
mod render;
mod state;

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
                  muku-pet --preview FILE.png [--action idle|pet|feed|play|sleep] [--time SECONDS]\n\
                  muku-pet --simulate pet,feed,play,sleep --state FILE\n\
                  muku-pet --inspect\n\
                  Device: buttons/d-pad care for muku; START saves and exits.\n\
                  No wall clock, network, firmware changes, or power-off penalties.");
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
        render::render(&state, t).save_png(&path)?;
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
            return device::run(&mut state, &state_path, seconds, snapshot.as_deref());
        }
    }
    Err("use --preview on macOS, or --device / --inspect on the Linux handheld".into())
}
