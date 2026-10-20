//! Screen flow and the small amount of state that belongs to a play session.

use crate::{
    minigame::{GameInput, GameKind, GameResult, MiniGame},
    notifications::Notice,
    state::{Action, PetState},
};

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    X,
    Y,
    Start,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Screen {
    Home,
    GameSelect,
    Playing,
    Result,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    None,
    Save,
    Exit,
}

pub struct App {
    pub pet: PetState,
    pub screen: Screen,
    pub selected_game: usize,
    pub game: Option<MiniGame>,
    pub result: Option<GameResult>,
    pub notice: Option<Notice>,
    pub notice_age: f32,
    total_time: f64,
}

impl App {
    pub fn new(pet: PetState) -> Self {
        Self {
            pet,
            screen: Screen::Home,
            selected_game: 0,
            game: None,
            result: None,
            notice: None,
            notice_age: 0.0,
            total_time: 0.0,
        }
    }

    pub fn tick(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.total_time += f64::from(dt);
        self.pet.tick(dt);
        if self.screen == Screen::Home && self.notice.is_some() {
            self.notice_age += dt;
            if self.notice_age >= 9.0 {
                self.notice = None;
                self.notice_age = 0.0;
            }
        }
        if self.screen == Screen::Playing {
            if let Some(game) = &mut self.game {
                game.tick(dt);
                if game.finished() {
                    self.finish_game();
                }
            }
        }
    }

    pub fn push_notice(&mut self, notice: Notice) {
        self.notice = Some(notice);
        self.notice_age = 0.0;
    }

    pub fn visible_notice(&self) -> Option<(&Notice, f32)> {
        (self.screen == Screen::Home && self.notice_age < 9.0)
            .then_some(self.notice.as_ref())
            .flatten()
            .map(|notice| (notice, self.notice_age))
    }

    pub fn handle(&mut self, button: Button) -> AppEvent {
        match self.screen {
            Screen::Home => self.handle_home(button),
            Screen::GameSelect => self.handle_select(button),
            Screen::Playing => self.handle_playing(button),
            Screen::Result => self.handle_result(button),
        }
    }

    fn handle_home(&mut self, button: Button) -> AppEvent {
        match button {
            Button::Up | Button::A => {
                self.pet.act(Action::Pet);
                AppEvent::Save
            }
            Button::Right | Button::B => {
                self.pet.act(Action::Feed);
                AppEvent::Save
            }
            Button::Down | Button::X => {
                self.screen = Screen::GameSelect;
                AppEvent::None
            }
            Button::Left | Button::Y => {
                self.pet.act(Action::Sleep);
                AppEvent::Save
            }
            Button::Start => AppEvent::Exit,
        }
    }

    fn handle_select(&mut self, button: Button) -> AppEvent {
        match button {
            Button::Left | Button::Up => {
                self.selected_game =
                    (self.selected_game + GameKind::ALL.len() - 1) % GameKind::ALL.len();
                AppEvent::None
            }
            Button::Right | Button::Down => {
                self.selected_game = (self.selected_game + 1) % GameKind::ALL.len();
                AppEvent::None
            }
            Button::A | Button::X => {
                self.start_selected();
                AppEvent::None
            }
            Button::B | Button::Y => {
                self.screen = Screen::Home;
                AppEvent::None
            }
            Button::Start => {
                self.screen = Screen::Home;
                AppEvent::None
            }
        }
    }

    fn handle_playing(&mut self, button: Button) -> AppEvent {
        if button == Button::Start {
            self.finish_game();
            return AppEvent::Save;
        }
        let input = match button {
            Button::Left => Some(GameInput::Left),
            Button::Right => Some(GameInput::Right),
            Button::A | Button::Up => Some(GameInput::Lane(0)),
            Button::B => Some(GameInput::Lane(1)),
            Button::X | Button::Down => Some(GameInput::Lane(2)),
            Button::Y => Some(GameInput::Lane(3)),
            Button::Start => unreachable!(),
        };
        if let (Some(game), Some(input)) = (&mut self.game, input) {
            game.input(input);
        }
        AppEvent::None
    }

    fn handle_result(&mut self, button: Button) -> AppEvent {
        match button {
            Button::A | Button::X => {
                self.start_selected();
                AppEvent::None
            }
            Button::B | Button::Y => {
                self.screen = Screen::GameSelect;
                self.result = None;
                AppEvent::None
            }
            Button::Start | Button::Up | Button::Down | Button::Left | Button::Right => {
                self.screen = Screen::Home;
                self.result = None;
                AppEvent::None
            }
        }
    }

    fn start_selected(&mut self) {
        let kind = GameKind::ALL[self.selected_game.min(GameKind::ALL.len() - 1)];
        let seed = ((self.total_time * 1_000.0) as u64)
            ^ self.pet.interactions.rotate_left(17)
            ^ u64::from(std::process::id());
        self.game = Some(MiniGame::new(kind, seed));
        self.result = None;
        self.screen = Screen::Playing;
    }

    fn finish_game(&mut self) {
        let Some(game) = &self.game else {
            self.screen = Screen::Home;
            return;
        };
        self.result = Some(game.result());
        self.pet.act(Action::Play);
        self.game = None;
        self.screen = Screen::Result;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minigame::MiniGame;

    #[test]
    fn x_opens_game_menu_and_b_returns_home() {
        let mut app = App::new(PetState::default());
        assert_eq!(app.handle(Button::X), AppEvent::None);
        assert_eq!(app.screen, Screen::GameSelect);
        app.handle(Button::B);
        assert_eq!(app.screen, Screen::Home);
    }

    #[test]
    fn selecting_starts_the_highlighted_game() {
        let mut app = App::new(PetState::default());
        app.handle(Button::X);
        app.handle(Button::Right);
        app.handle(Button::A);
        assert_eq!(app.screen, Screen::Playing);
        assert!(matches!(app.game.as_ref(), Some(MiniGame::Beat(_))));
    }

    #[test]
    fn leaving_a_game_rewards_play_once() {
        let mut app = App::new(PetState::default());
        app.handle(Button::X);
        app.handle(Button::A);
        let before = app.pet.joy;
        app.handle(Button::Start);
        assert_eq!(app.screen, Screen::Result);
        assert!(app.pet.joy > before);
        assert!(app.result.is_some());
    }

    #[test]
    fn notice_waits_for_home_and_then_expires() {
        let mut app = App::new(PetState::default());
        app.screen = Screen::Playing;
        app.push_notice(Notice {
            id: "one".into(),
            title: "hello".into(),
            body: "from ai".into(),
            source: "ai".into(),
            kind: "notice".into(),
            audio_wav_base64: None,
        });
        app.tick(20.0);
        assert!(app.notice.is_some());
        app.screen = Screen::Home;
        app.tick(9.0);
        assert!(app.notice.is_none());
    }
}
