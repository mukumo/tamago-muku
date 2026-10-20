//! Muku's offline needs and persistence.
//!
//! All needs use a 0–100 scale. Only `tick` advances time: loading a save never
//! reads a clock or charges Muku for time spent with the device switched off.

use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const SAVE_VERSION: u32 = 1;
const MAX_SAVE_BYTES: u64 = 64 * 1024;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    Idle,
    Pet,
    Feed,
    Play,
    Sleep,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PetState {
    pub version: u32,
    pub fullness: f32,
    pub joy: f32,
    pub energy: f32,
    pub asleep: bool,
    pub total_awake_secs: f64,
    pub interactions: u64,
    pub last_action: Action,
    pub action_age: f32,
}

impl Default for PetState {
    fn default() -> Self {
        Self {
            version: SAVE_VERSION,
            fullness: 80.0,
            joy: 75.0,
            energy: 90.0,
            asleep: false,
            total_awake_secs: 0.0,
            interactions: 0,
            last_action: Action::Idle,
            action_age: 0.0,
        }
    }
}

impl PetState {
    /// Perform an interaction. Sleep toggles; care and play gently wake Muku.
    pub fn act(&mut self, action: Action) {
        match action {
            Action::Idle => {}
            Action::Pet => {
                self.asleep = false;
                self.joy += 8.0;
                self.energy += 1.0;
            }
            Action::Feed => {
                self.asleep = false;
                self.fullness += 22.0;
                self.joy += 3.0;
            }
            Action::Play => {
                self.asleep = false;
                self.joy += 16.0;
                self.energy -= 8.0;
                self.fullness -= 3.0;
            }
            Action::Sleep => {
                self.asleep = !self.asleep;
            }
        }
        if action != Action::Idle {
            self.interactions = self.interactions.saturating_add(1);
        }
        self.fullness = self.fullness.clamp(0.0, 100.0);
        self.joy = self.joy.clamp(0.0, 100.0);
        self.energy = self.energy.clamp(0.0, 100.0);
        self.last_action = action;
        self.action_age = 0.0;
    }

    /// Advance by active elapsed seconds, with no death, illness, or penalties
    /// tied to the wall clock. Invalid or negative durations are ignored.
    pub fn tick(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let seconds = f64::from(dt);
        let hours = seconds / 3_600.0;
        let (fullness, joy, energy) = if self.asleep {
            (
                f64::from(self.fullness) - hours * 5.0,
                f64::from(self.joy),
                f64::from(self.energy) + hours * 50.0,
            )
        } else {
            self.total_awake_secs = (self.total_awake_secs + seconds).min(f64::MAX);
            (
                f64::from(self.fullness) - hours * 10.0,
                f64::from(self.joy) - hours * (100.0 / 12.0),
                f64::from(self.energy) - hours * 6.25,
            )
        };
        self.fullness = fullness.clamp(0.0, 100.0) as f32;
        self.joy = joy.clamp(0.0, 100.0) as f32;
        self.energy = energy.clamp(0.0, 100.0) as f32;
        self.action_age = (f64::from(self.action_age) + seconds).min(f64::from(f32::MAX)) as f32;
    }

    /// Atomically replace a save after validation, a complete write, and fsync.
    ///
    /// The temporary file lives alongside the destination so rename stays on
    /// the same filesystem. Any failure before rename leaves the old save
    /// intact. There are no fallible operations after a successful rename.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(invalid_data)?;
        let filename = path.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "save path needs a filename")
        })?;
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        let (temp, mut file) = create_temp(parent, filename)?;
        let result = (|| {
            file.write_all(&bytes)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temp.0, path)
        })();
        // The guard removes incomplete files; after rename its path is absent.
        drop(temp);
        result
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut bytes = Vec::new();
        file.take(MAX_SAVE_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_SAVE_BYTES {
            return Err(invalid_data("save file is too large"));
        }
        let state: Self = serde_json::from_slice(&bytes).map_err(invalid_data)?;
        state.validate()?;
        Ok(state)
    }

    fn validate(&self) -> io::Result<()> {
        if self.version != SAVE_VERSION {
            return Err(invalid_data("unsupported save version"));
        }
        for value in [self.fullness, self.joy, self.energy] {
            if !value.is_finite() || !(0.0..=100.0).contains(&value) {
                return Err(invalid_data(
                    "save needs must be finite values from 0 to 100",
                ));
            }
        }
        if !self.total_awake_secs.is_finite() || self.total_awake_secs < 0.0 {
            return Err(invalid_data(
                "save awake duration must be finite and nonnegative",
            ));
        }
        if !self.action_age.is_finite() || self.action_age < 0.0 {
            return Err(invalid_data(
                "save action age must be finite and nonnegative",
            ));
        }
        Ok(())
    }
}

fn invalid_data(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn create_temp(parent: &Path, filename: &std::ffi::OsStr) -> io::Result<(TempFile, File)> {
    for _ in 0..32 {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let mut name = OsString::from(".");
        name.push(filename);
        name.push(format!(".{}.{}.tmp", std::process::id(), id));
        let path = parent.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((TempFile(path), file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not reserve a temporary save file",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("muku-state-{}-{id}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn care_and_play_have_distinct_bounded_effects() {
        let mut pet = PetState::default();
        pet.act(Action::Feed);
        assert_eq!(pet.fullness, 100.0);
        assert_eq!(pet.joy, 78.0);
        pet.act(Action::Play);
        assert_eq!(pet.fullness, 97.0);
        assert_eq!(pet.joy, 94.0);
        assert_eq!(pet.energy, 82.0);
        pet.act(Action::Pet);
        assert_eq!(pet.joy, 100.0);
        assert_eq!(pet.energy, 83.0);
        assert_eq!(pet.interactions, 3);
        assert_eq!(pet.last_action, Action::Pet);
        assert_eq!(pet.action_age, 0.0);
    }

    #[test]
    fn sleep_recovers_energy_and_care_wakes() {
        let mut pet = PetState {
            energy: 20.0,
            ..PetState::default()
        };
        pet.act(Action::Sleep);
        pet.tick(3_600.0);
        assert!(pet.asleep);
        assert_eq!(pet.energy, 70.0);
        assert_eq!(pet.fullness, 75.0);
        assert_eq!(pet.joy, 75.0);
        assert_eq!(pet.total_awake_secs, 0.0);
        pet.act(Action::Sleep);
        assert!(!pet.asleep);
        pet.act(Action::Sleep);
        pet.act(Action::Pet);
        assert!(!pet.asleep);
        pet.tick(60.0);
        assert_eq!(pet.total_awake_secs, 60.0);
        assert_eq!(pet.action_age, 60.0);
    }

    #[test]
    fn needs_drift_gently_during_an_active_hour() {
        let mut pet = PetState::default();
        pet.tick(3_600.0);
        assert_eq!(pet.fullness, 70.0);
        assert!((pet.joy - 66.666_664).abs() < 0.001);
        assert_eq!(pet.energy, 83.75);
        assert_eq!(pet.total_awake_secs, 3_600.0);
    }

    #[test]
    fn long_durations_clamp_without_overflow_or_death() {
        let mut pet = PetState::default();
        pet.tick(f32::MAX);
        pet.tick(f32::MAX);
        assert_eq!((pet.fullness, pet.joy, pet.energy), (0.0, 0.0, 0.0));
        assert!(pet.total_awake_secs.is_finite());
        assert_eq!(pet.action_age, f32::MAX);
        assert!(pet.validate().is_ok());
        pet.act(Action::Feed);
        pet.act(Action::Sleep);
        pet.tick(f32::MAX);
        assert_eq!(pet.energy, 100.0);
        assert!(pet.validate().is_ok());
    }

    #[test]
    fn invalid_durations_do_not_advance_state() {
        let mut pet = PetState::default();
        let before = serde_json::to_value(&pet).unwrap();
        for duration in [-1.0, 0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            pet.tick(duration);
        }
        assert_eq!(serde_json::to_value(&pet).unwrap(), before);
    }

    #[test]
    fn save_load_roundtrip_does_not_advance_time() {
        let directory = TestDir::new();
        let path = directory.0.join("nested/muku.json");
        let mut pet = PetState::default();
        pet.act(Action::Play);
        pet.tick(12.5);
        pet.save(&path).unwrap();
        let restored = PetState::load(&path).unwrap();
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(&pet).unwrap()
        );
        assert_eq!(restored.total_awake_secs, 12.5);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn malformed_versions_and_invalid_numbers_are_rejected() {
        let directory = TestDir::new();
        let path = directory.0.join("muku.json");
        fs::write(&path, b"{not json").unwrap();
        assert_eq!(
            PetState::load(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        for (field, value) in [
            ("version", serde_json::json!(2)),
            ("fullness", serde_json::json!(101)),
            ("joy", serde_json::json!(-1)),
            ("energy", serde_json::Value::Null),
            ("total_awake_secs", serde_json::json!(-0.1)),
            ("action_age", serde_json::json!(-1)),
        ] {
            let mut data = serde_json::to_value(PetState::default()).unwrap();
            data[field] = value;
            fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
            assert_eq!(
                PetState::load(&path).unwrap_err().kind(),
                io::ErrorKind::InvalidData,
                "field {field}"
            );
        }
    }

    #[test]
    fn invalid_state_cannot_overwrite_an_existing_save() {
        let directory = TestDir::new();
        let path = directory.0.join("muku.json");
        let mut pet = PetState::default();
        pet.save(&path).unwrap();
        let before = fs::read(&path).unwrap();
        pet.energy = f32::NAN;
        assert_eq!(
            pet.save(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }

    #[test]
    fn failed_rename_cleans_up_the_temporary_file() {
        let directory = TestDir::new();
        let path = directory.0.join("muku.json");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("keep"), b"existing destination").unwrap();
        assert!(PetState::default().save(&path).is_err());
        assert_eq!(
            fs::read(path.join("keep")).unwrap(),
            b"existing destination"
        );
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }
}
