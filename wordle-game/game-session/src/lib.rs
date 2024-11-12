#![no_std]

use game_session_io::*;
use gstd::{exec, msg};

const TRIES_LIMIT: u8 = 5;

static mut GAME_SESSION_STATE: Option<GameSession> = None;

#[no_mangle]
extern "C" fn init() {
    let game_session_init: GameSessionInit =
        msg::load().expect("Unable to decode `GameSessionInit`");
    if !game_session_init.validate() {
        panic!("{}", game_session_init.validation_error());
    }
    unsafe { GAME_SESSION_STATE = Some(game_session_init.into()) };
}

#[no_mangle]
extern "C" fn handle() {
    let game_session_action: GameSessionAction =
        msg::load().expect("Unable to decode `GameSessionAction`");
    let game_session = unsafe {
        GAME_SESSION_STATE
            .as_mut()
            .expect("Game is not initialized")
    };
    match game_session_action {
        GameSessionAction::StartGame => {
            let user = msg::source();
            let wordle_id = game_session.wordle_program_id();
            let session_info = game_session.get_or_create_session(user);
            match &session_info.session_status {
                SessionStatus::ReplyReceived(wordle_event) => {
                    msg::reply::<GameSessionEvent>(wordle_event.into(), 0)
                        .expect("Error in sending a reply");
                    session_info.session_status = SessionStatus::InProgress;
                }
                _ if session_info.can_start_new_game() => {
                    let send_to_wordle_msg_id =
                        msg::send(wordle_id, WordleAction::new_start_game(user), 0)
                            .expect("Error in sending a message");

                    session_info.session_id = msg::id();
                    session_info.update_message_ids(msg::id(), send_to_wordle_msg_id);
                    session_info.tries = 0;
                    session_info.session_status = SessionStatus::InProgress;
                    msg::send_delayed(
                        exec::program_id(),
                        GameSessionAction::CheckGameStatus {
                            user,
                            session_id: msg::id(),
                        },
                        0,
                        200,
                    )
                    .expect("Error in send_delayed a message");
                    exec::wait();
                }
                _ => {
                    panic!("The user is in the game");
                }
            }
        }
        GameSessionAction::CheckWord(word) => {
            let user = msg::source();
            let wordle_id = game_session.wordle_program_id();
            let session_info = game_session.get_or_create_session(user);
            match &session_info.session_status {
                SessionStatus::ReplyReceived(wordle_event) => {
                    let event_copy = wordle_event.clone();
                    let has_guessed = event_copy.has_guessed();
                    if session_info.increment_tries() == TRIES_LIMIT {
                        session_info.session_status = SessionStatus::GameOver(GameStatus::Lose);
                        msg::reply(GameSessionEvent::game_over(GameStatus::Lose), 0)
                            .expect("Error in sending a reply");
                    } else if has_guessed {
                        session_info.session_status = SessionStatus::GameOver(GameStatus::Win);
                        msg::reply(GameSessionEvent::game_over(GameStatus::Win), 0)
                            .expect("Error in sending a reply");
                    } else {
                        msg::reply::<GameSessionEvent>((&event_copy).into(), 0)
                            .expect("Error in sending a reply");
                        session_info.session_status = SessionStatus::InProgress;
                    }
                }
                SessionStatus::InProgress => {
                    GameSession::validate_word(&word).expect("Invalid word");
                    let send_to_wordle_msg_id =
                        msg::send(wordle_id, WordleAction::new_check_word(user, word), 0)
                            .expect("Error in sending a message");
                    session_info.update_message_ids(msg::id(), send_to_wordle_msg_id);
                    exec::wait();
                }
                _ => {
                    panic!("The user is not in the game");
                }
            }
        }
        GameSessionAction::CheckGameStatus { user, session_id } => {
            if msg::source() == exec::program_id() {
                if let Some(session_info) = game_session.sessions.get_mut(&user) {
                    if session_id == session_info.session_id
                        && !matches!(session_info.session_status, SessionStatus::GameOver(..))
                    {
                        session_info.session_status = SessionStatus::GameOver(GameStatus::Lose);
                        msg::send(user, GameSessionEvent::GameOver(GameStatus::Lose), 0)
                            .expect("Error in sending a reply");
                    }
                }
            }
        }
    }
}

#[no_mangle]
extern "C" fn handle_reply() {
    let reply_to = msg::reply_to().expect("Failed to query reply_to data");
    let wordle_event: WordleEvent = msg::load().expect("Unable to decode `WordleEvent`");
    let game_session = unsafe {
        GAME_SESSION_STATE
            .as_mut()
            .expect("Game is not initialized")
    };
    let user = wordle_event.get_user();
    if let Some(session_info) = game_session.sessions.get_mut(user) {
        if reply_to == session_info.send_to_wordle_msg_id && session_info.is_wait_reply_status() {
            session_info.session_status = SessionStatus::ReplyReceived(wordle_event);
            exec::wake(session_info.original_msg_id).expect("Failed to wake message");
        }
    }
}

#[no_mangle]
extern "C" fn state() {
    let game_session = unsafe {
        GAME_SESSION_STATE
            .as_ref()
            .expect("Game is not initialized")
    };
    msg::reply::<GameSessionState>(game_session.into(), 0)
        .expect("failed to encode or reply from `state()`");
}
