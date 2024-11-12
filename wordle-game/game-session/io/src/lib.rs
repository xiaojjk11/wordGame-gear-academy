#![no_std]

use gmeta::{In, InOut, Metadata, Out};
use gstd::{collections::HashMap, prelude::*, ActorId, MessageId};

pub struct GameSessionMetadata;

impl Metadata for GameSessionMetadata {
    type Init = In<GameSessionInit>;
    type Handle = InOut<GameSessionAction, GameSessionEvent>;
    type Reply = ();
    type Others = ();
    type Signal = ();
    type State = Out<GameSessionState>;
}

pub trait GameResult {
    fn is_win(&self) -> bool;
    fn is_lose(&self) -> bool;
}

impl GameResult for GameStatus {
    fn is_win(&self) -> bool {
        matches!(self, GameStatus::Win)
    }

    fn is_lose(&self) -> bool {
        matches!(self, GameStatus::Lose)
    }
}

pub trait Validatable {
    fn validate(&self) -> bool;
    fn validation_error(&self) -> &'static str;
}

#[derive(Debug, Default, Clone, Encode, Decode, TypeInfo)]
pub struct GameSessionState {
    pub wordle_program_id: ActorId,
    pub game_sessions: Vec<(ActorId, SessionInfo)>,
}

#[derive(Debug, Default, Clone, Encode, Decode, TypeInfo)]
pub struct GameSessionInit {
    pub wordle_program_id: ActorId,
}

impl Validatable for GameSessionInit {
    fn validate(&self) -> bool {
        !self.wordle_program_id.is_zero()
    }

    fn validation_error(&self) -> &'static str {
        "Invalid wordle_program_id"
    }
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum GameSessionAction {
    StartGame,
    CheckWord(String),
    CheckGameStatus {
        user: ActorId,
        session_id: MessageId,
    },
}

impl GameSessionAction {
    pub fn is_timeout_check(&self) -> bool {
        matches!(self, Self::CheckGameStatus { .. })
    }
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum WordleAction {
    StartGame { user: ActorId },
    CheckWord { user: ActorId, word: String },
}

impl WordleAction {
    pub fn new_start_game(user: ActorId) -> Self {
        Self::StartGame { user }
    }

    pub fn new_check_word(user: ActorId, word: String) -> Self {
        Self::CheckWord { user, word }
    }
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum GameSessionEvent {
    StartSuccess,
    CheckWordResult {
        correct_positions: Vec<u8>,
        contained_in_word: Vec<u8>,
        #[codec(skip)]
        positions: Vec<u8>,
        #[codec(skip)]
        contains: Vec<u8>,
    },
    GameOver(GameStatus),
}

impl GameSessionEvent {
    pub fn game_over(status: GameStatus) -> Self {
        Self::GameOver(status)
    }
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum GameStatus {
    Win,
    Lose,
}

#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
pub enum WordleEvent {
    GameStarted {
        user: ActorId,
    },
    WordChecked {
        user: ActorId,
        correct_positions: Vec<u8>,
        contained_in_word: Vec<u8>,
    },
}

impl WordleEvent {
    pub fn get_user(&self) -> &ActorId {
        match self {
            Self::GameStarted { user } | Self::WordChecked { user, .. } => user,
        }
    }

    pub fn has_guessed(&self) -> bool {
        match self {
            Self::WordChecked {
                correct_positions, ..
            } => correct_positions.len() == 5 && correct_positions.iter().all(|&pos| pos < 5),
            _ => false,
        }
    }
}

impl From<&WordleEvent> for GameSessionEvent {
    fn from(wordle_event: &WordleEvent) -> Self {
        match wordle_event {
            WordleEvent::GameStarted { .. } => GameSessionEvent::StartSuccess,
            WordleEvent::WordChecked {
                correct_positions,
                contained_in_word,
                ..
            } => GameSessionEvent::CheckWordResult {
                correct_positions: correct_positions.clone(),
                contained_in_word: contained_in_word.clone(),
                positions: correct_positions.clone(),
                contains: contained_in_word.clone(),
            },
        }
    }
}

#[derive(Default, Debug, Clone, Encode, Decode, TypeInfo)]
pub enum SessionStatus {
    #[default]
    Init,
    InProgress,
    ReplyReceived(WordleEvent),
    GameOver(GameStatus),
}

impl SessionStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::InProgress | Self::ReplyReceived(_))
    }
}

#[derive(Default, Debug, Clone, Encode, Decode, TypeInfo)]
pub struct SessionInfo {
    pub session_id: MessageId,
    pub original_msg_id: MessageId,
    pub send_to_wordle_msg_id: MessageId,
    pub tries: u8,
    pub session_status: SessionStatus,
}

impl SessionInfo {
    pub fn create(msg_id: MessageId) -> Self {
        Self {
            session_id: msg_id,
            original_msg_id: msg_id,
            send_to_wordle_msg_id: msg_id,
            ..Default::default()
        }
    }

    pub fn update_message_ids(&mut self, original: MessageId, send_to_wordle: MessageId) {
        self.original_msg_id = original;
        self.send_to_wordle_msg_id = send_to_wordle;
    }

    pub fn is_wait_reply_status(&self) -> bool {
        matches!(self.session_status, SessionStatus::InProgress)
    }

    pub fn is_game_over(&self) -> bool {
        matches!(self.session_status, SessionStatus::GameOver(_))
    }

    pub fn can_start_new_game(&self) -> bool {
        self.is_init() || self.is_game_over()
    }

    pub fn is_init(&self) -> bool {
        matches!(self.session_status, SessionStatus::Init)
    }

    pub fn has_reply(&self) -> bool {
        matches!(self.session_status, SessionStatus::ReplyReceived(_))
    }

    pub fn take_reply(&mut self) -> Option<WordleEvent> {
        if let SessionStatus::ReplyReceived(event) =
            mem::replace(&mut self.session_status, SessionStatus::InProgress)
        {
            Some(event)
        } else {
            None
        }
    }

    pub fn increment_tries(&mut self) -> u8 {
        self.tries += 1;
        self.tries
    }
}

#[derive(Default, Debug, Clone)]
pub struct GameSession {
    pub wordle_program_id: ActorId,
    pub sessions: HashMap<ActorId, SessionInfo>,
}

impl GameSession {
    pub fn get_or_create_session(&mut self, user: ActorId) -> &mut SessionInfo {
        self.sessions.entry(user).or_default()
    }

    pub fn wordle_program_id(&self) -> ActorId {
        self.wordle_program_id
    }

    pub fn validate_word(word: &str) -> Result<(), &'static str> {
        if word.len() != 5 {
            return Err("Word must be 5 characters long");
        }
        if !word.chars().all(|c| c.is_lowercase()) {
            return Err("Word must contain only lowercase letters");
        }
        Ok(())
    }
}

impl From<GameSessionInit> for GameSession {
    fn from(init: GameSessionInit) -> Self {
        Self {
            wordle_program_id: init.wordle_program_id,
            ..Default::default()
        }
    }
}

impl From<&GameSession> for GameSessionState {
    fn from(game_session: &GameSession) -> Self {
        Self {
            wordle_program_id: game_session.wordle_program_id,
            game_sessions: game_session
                .sessions
                .iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
        }
    }
}
