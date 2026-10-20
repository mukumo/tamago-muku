//! Small deterministic games that use the handheld's physical controls.

use serde::{Deserialize, Serialize};

const STAR_DURATION: f32 = 25.0;
const STAR_FIELD_LEFT: f32 = 64.0;
const STAR_FIELD_RIGHT: f32 = 416.0;
const STAR_LINE: f32 = 194.0;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameKind {
    StarCatch,
    BeatGarden,
}

impl GameKind {
    pub const ALL: [Self; 2] = [Self::StarCatch, Self::BeatGarden];

    pub fn title(self) -> &'static str {
        match self {
            Self::StarCatch => "STAR CATCH",
            Self::BeatGarden => "BEAT GARDEN",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::StarCatch => "move muku. catch the falling stars.",
            Self::BeatGarden => "tap A B X Y when notes reach the line.",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GameInput {
    Left,
    Right,
    Lane(usize),
}

#[derive(Clone, Debug)]
pub struct FallingStar {
    pub x: f32,
    pub y: f32,
    pub speed: f32,
    pub gold: bool,
}

#[derive(Clone, Debug)]
pub struct StarCatch {
    pub basket_x: f32,
    pub stars: Vec<FallingStar>,
    pub caught: u32,
    pub missed: u32,
    pub combo: u32,
    pub best_combo: u32,
    pub remaining: f32,
    spawn_in: f32,
    rng: u64,
}

impl StarCatch {
    fn new(seed: u64) -> Self {
        Self {
            basket_x: 240.0,
            stars: Vec::new(),
            caught: 0,
            missed: 0,
            combo: 0,
            best_combo: 0,
            remaining: STAR_DURATION,
            spawn_in: 0.35,
            rng: seed.max(1),
        }
    }

    pub fn score(&self) -> u32 {
        self.caught
    }

    fn random(&mut self) -> f32 {
        // A compact deterministic generator keeps previews and tests stable.
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.rng >> 40) as f32) / ((1_u32 << 24) as f32)
    }

    fn input(&mut self, input: GameInput) {
        match input {
            GameInput::Left => self.basket_x -= 30.0,
            GameInput::Right => self.basket_x += 30.0,
            GameInput::Lane(_) => {}
        }
        self.basket_x = self.basket_x.clamp(STAR_FIELD_LEFT, STAR_FIELD_RIGHT);
    }

    fn tick(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let dt = dt.min(0.1);
        self.remaining = (self.remaining - dt).max(0.0);
        self.spawn_in -= dt;
        while self.spawn_in <= 0.0 {
            let x = STAR_FIELD_LEFT + self.random() * (STAR_FIELD_RIGHT - STAR_FIELD_LEFT);
            let progress = 1.0 - self.remaining / STAR_DURATION;
            let speed = 68.0 + progress * 42.0 + self.random() * 18.0;
            let gold = self.random() > 0.82;
            self.stars.push(FallingStar {
                x,
                y: 75.0,
                speed,
                gold,
            });
            self.spawn_in += 0.58 + self.random() * 0.48;
        }
        let mut missed_this_tick = 0;
        for star in &mut self.stars {
            star.y += star.speed * dt;
            if star.y < STAR_LINE {
                continue;
            }
            if (star.x - self.basket_x).abs() <= if star.gold { 39.0 } else { 33.0 } {
                self.caught += if star.gold { 3 } else { 1 };
                self.combo = self.combo.saturating_add(1);
                self.best_combo = self.best_combo.max(self.combo);
            } else {
                self.missed += 1;
                missed_this_tick += 1;
                self.combo = 0;
            }
            star.y = f32::INFINITY;
        }
        self.stars.retain(|star| star.y.is_finite());
        if missed_this_tick >= 2 {
            self.combo = 0;
        }
    }

    fn finished(&self) -> bool {
        self.remaining <= 0.0 || self.missed >= 6
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NoteState {
    Waiting,
    Hit,
    Missed,
}

#[derive(Clone, Debug)]
pub struct BeatNote {
    pub lane: usize,
    pub time: f32,
    pub state: NoteState,
    pub exact: bool,
}

#[derive(Clone, Debug)]
pub struct BeatGarden {
    pub notes: Vec<BeatNote>,
    pub elapsed: f32,
    pub score: u32,
    pub combo: u32,
    pub best_combo: u32,
    pub hits: u32,
    pub misses: u32,
    pub feedback: Option<(bool, f32)>,
}

impl BeatGarden {
    fn new() -> Self {
        const PATTERN: [usize; 28] = [
            0, 1, 2, 3, 1, 0, 2, 3, 0, 2, 1, 3, 3, 1, 0, 2, 0, 1, 2, 3, 2, 1, 3, 0, 1, 2, 0, 3,
        ];
        let notes = PATTERN
            .into_iter()
            .enumerate()
            .map(|(i, lane)| BeatNote {
                lane,
                time: 1.7 + i as f32 * 0.47,
                state: NoteState::Waiting,
                exact: false,
            })
            .collect();
        Self {
            notes,
            elapsed: 0.0,
            score: 0,
            combo: 0,
            best_combo: 0,
            hits: 0,
            misses: 0,
            feedback: None,
        }
    }

    pub fn duration(&self) -> f32 {
        self.notes.last().map_or(0.0, |note| note.time) + 1.15
    }

    pub fn remaining(&self) -> f32 {
        (self.duration() - self.elapsed).max(0.0)
    }

    fn input(&mut self, input: GameInput) {
        let GameInput::Lane(lane) = input else {
            return;
        };
        if lane > 3 {
            return;
        }
        let closest = self
            .notes
            .iter()
            .enumerate()
            .filter(|(_, note)| note.state == NoteState::Waiting && note.lane == lane)
            .map(|(index, note)| (index, (note.time - self.elapsed).abs()))
            .filter(|(_, delay)| *delay <= 0.26)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let Some((index, delay)) = closest else {
            self.combo = 0;
            self.misses = self.misses.saturating_add(1);
            self.feedback = Some((false, self.elapsed));
            return;
        };
        let note = &mut self.notes[index];
        note.state = NoteState::Hit;
        note.exact = delay <= 0.095;
        self.hits = self.hits.saturating_add(1);
        self.combo = self.combo.saturating_add(1);
        self.best_combo = self.best_combo.max(self.combo);
        self.score = self
            .score
            .saturating_add(if note.exact { 300 } else { 150 })
            .saturating_add(self.combo.min(20) * 5);
        self.feedback = Some((true, self.elapsed));
    }

    fn tick(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.elapsed += dt.min(0.1);
        for note in &mut self.notes {
            if note.state == NoteState::Waiting && self.elapsed - note.time > 0.26 {
                note.state = NoteState::Missed;
                self.misses = self.misses.saturating_add(1);
                self.combo = 0;
            }
        }
    }

    fn finished(&self) -> bool {
        self.elapsed >= self.duration()
    }
}

#[derive(Clone, Debug)]
pub enum MiniGame {
    Star(StarCatch),
    Beat(BeatGarden),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GameResult {
    pub kind: GameKind,
    pub score: u32,
    pub best_combo: u32,
    pub hits: u32,
    pub misses: u32,
}

impl GameResult {
    pub fn stars(self) -> usize {
        match self.kind {
            GameKind::StarCatch => match self.score {
                0..=4 => 1,
                5..=9 => 2,
                10..=16 => 3,
                17..=24 => 4,
                _ => 5,
            },
            GameKind::BeatGarden => match self.score {
                0..=2_499 => 1,
                2_500..=4_999 => 2,
                5_000..=7_499 => 3,
                7_500..=9_999 => 4,
                _ => 5,
            },
        }
    }
}

impl MiniGame {
    pub fn new(kind: GameKind, seed: u64) -> Self {
        match kind {
            GameKind::StarCatch => Self::Star(StarCatch::new(seed)),
            GameKind::BeatGarden => Self::Beat(BeatGarden::new()),
        }
    }

    pub fn input(&mut self, input: GameInput) {
        match self {
            Self::Star(game) => game.input(input),
            Self::Beat(game) => game.input(input),
        }
    }

    pub fn tick(&mut self, dt: f32) {
        match self {
            Self::Star(game) => game.tick(dt),
            Self::Beat(game) => game.tick(dt),
        }
    }

    pub fn finished(&self) -> bool {
        match self {
            Self::Star(game) => game.finished(),
            Self::Beat(game) => game.finished(),
        }
    }

    pub fn result(&self) -> GameResult {
        match self {
            Self::Star(game) => GameResult {
                kind: GameKind::StarCatch,
                score: game.score(),
                best_combo: game.best_combo,
                hits: game.caught,
                misses: game.missed,
            },
            Self::Beat(game) => GameResult {
                kind: GameKind::BeatGarden,
                score: game.score,
                best_combo: game.best_combo,
                hits: game.hits,
                misses: game.misses,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advance(game: &mut BeatGarden, seconds: f32) {
        for _ in 0..(seconds / 0.05).round() as usize {
            game.tick(0.05);
        }
    }

    #[test]
    fn star_catch_moves_only_inside_the_field() {
        let mut game = StarCatch::new(7);
        for _ in 0..30 {
            game.input(GameInput::Left);
        }
        assert_eq!(game.basket_x, STAR_FIELD_LEFT);
        for _ in 0..30 {
            game.input(GameInput::Right);
        }
        assert_eq!(game.basket_x, STAR_FIELD_RIGHT);
    }

    #[test]
    fn star_catch_spawns_catches_and_finishes() {
        let mut game = StarCatch::new(3);
        game.basket_x = 240.0;
        for _ in 0..300 {
            game.tick(0.1);
            for star in &game.stars {
                if star.y < 170.0 {
                    game.basket_x = star.x.clamp(STAR_FIELD_LEFT, STAR_FIELD_RIGHT);
                }
            }
            if game.finished() {
                break;
            }
        }
        assert!(game.finished());
        assert!(game.caught > 0);
    }

    #[test]
    fn beat_notes_are_hit_or_missed_once() {
        let mut game = BeatGarden::new();
        advance(&mut game, 1.7);
        let lane = game.notes[0].lane;
        game.input(GameInput::Lane(lane));
        assert_eq!(game.notes[0].state, NoteState::Hit);
        assert_eq!(game.hits, 1);
        assert!(game.score > 0);
        advance(&mut game, 15.0);
        assert!(game
            .notes
            .iter()
            .any(|note| note.state == NoteState::Missed));
        assert!(game.misses > 0);
        assert!(game.finished());
    }

    #[test]
    fn perfect_hit_scores_more_than_late_hit() {
        let mut perfect = BeatGarden::new();
        advance(&mut perfect, 1.7);
        perfect.input(GameInput::Lane(perfect.notes[0].lane));
        let mut late = BeatGarden::new();
        advance(&mut late, 1.9);
        late.input(GameInput::Lane(late.notes[0].lane));
        assert!(perfect.notes[0].exact);
        assert!(!late.notes[0].exact);
        assert!(perfect.score > late.score);
    }

    #[test]
    fn results_are_bounded_to_five_stars() {
        let result = GameResult {
            kind: GameKind::BeatGarden,
            score: u32::MAX,
            best_combo: 40,
            hits: 40,
            misses: 0,
        };
        assert_eq!(result.stars(), 5);
    }
}
