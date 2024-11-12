use game_session_io::*;
use gtest::{Log, ProgramBuilder, System};

const GAME_SESSION_PROGRAM_ID: u64 = 1;
const WORDLE_PROGRAM_ID: u64 = 2;
const USER: u64 = 3;

#[test]
fn test_basic_game_flow() {
    let system = System::new();
    system.init_logger();

    let game_session =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/game_session.opt.wasm")
            .with_id(GAME_SESSION_PROGRAM_ID)
            .build(&system);

    let wordle =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
            .with_id(WORDLE_PROGRAM_ID)
            .build(&system);

    let result = wordle.send_bytes(USER, []);
    assert!(!result.main_failed());

    let result = game_session.send(
        USER,
        GameSessionInit {
            wordle_program_id: WORDLE_PROGRAM_ID.into(),
        },
    );
    assert!(!result.main_failed());

    let result = game_session.send(USER, GameSessionAction::StartGame);
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::StartSuccess);
    assert!(!result.main_failed() && result.contains(&log));

    let result = game_session.send(USER, GameSessionAction::CheckWord("ab@cd".to_string()));
    assert!(result.main_failed());

    let result = game_session.send(USER, GameSessionAction::CheckWord("horse".to_string()));
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::GameOver(GameStatus::Win));
    assert!(!result.main_failed() && result.contains(&log));
}

#[test]
fn test_game_timeout() {
    let system = System::new();
    system.init_logger();

    let game_session =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/game_session.opt.wasm")
            .with_id(GAME_SESSION_PROGRAM_ID)
            .build(&system);

    let wordle =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
            .with_id(WORDLE_PROGRAM_ID)
            .build(&system);

    let result = wordle.send_bytes(USER, []);
    assert!(!result.main_failed());

    let result = game_session.send(
        USER,
        GameSessionInit {
            wordle_program_id: WORDLE_PROGRAM_ID.into(),
        },
    );
    assert!(!result.main_failed());

    let result = game_session.send(USER, GameSessionAction::StartGame);
    assert!(!result.main_failed());

    let result = system.spend_blocks(200);
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::GameOver(GameStatus::Lose));
    assert!(result[0].contains(&log));
}

#[test]
fn test_invalid_words() {
    let system = System::new();
    system.init_logger();

    let game_session =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/game_session.opt.wasm")
            .with_id(GAME_SESSION_PROGRAM_ID)
            .build(&system);

    let wordle =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
            .with_id(WORDLE_PROGRAM_ID)
            .build(&system);

    let result = wordle.send_bytes(USER, []);
    assert!(!result.main_failed());

    let result = game_session.send(
        USER,
        GameSessionInit {
            wordle_program_id: WORDLE_PROGRAM_ID.into(),
        },
    );
    assert!(!result.main_failed());

    let result = game_session.send(USER, GameSessionAction::StartGame);
    assert!(!result.main_failed());

    // 测试无效单词
    let result = game_session.send(USER, GameSessionAction::CheckWord("ab@cd".to_string()));
    assert!(result.main_failed());

    // 测试长度不正确的单词
    let result = game_session.send(USER, GameSessionAction::CheckWord("abcd".to_string()));
    assert!(result.main_failed());

    let result = game_session.send(USER, GameSessionAction::CheckWord("abcdef".to_string()));
    assert!(result.main_failed());
}

#[test]
fn test_multiple_guesses() {
    let system = System::new();
    system.init_logger();

    let game_session =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/game_session.opt.wasm")
            .with_id(GAME_SESSION_PROGRAM_ID)
            .build(&system);

    let wordle =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
            .with_id(WORDLE_PROGRAM_ID)
            .build(&system);

    let result = wordle.send_bytes(USER, []);
    assert!(!result.main_failed());

    let result = game_session.send(
        USER,
        GameSessionInit {
            wordle_program_id: WORDLE_PROGRAM_ID.into(),
        },
    );
    assert!(!result.main_failed());

    let result = game_session.send(USER, GameSessionAction::StartGame);
    assert!(!result.main_failed());

    // 多次尝试猜词
    let words = ["table", "chair", "mouse", "phone", "horse"];
    for word in words {
        let result = game_session.send(USER, GameSessionAction::CheckWord(word.to_string()));
        assert!(!result.main_failed());
    }
}

#[test]
fn test_restart_game() {
    let system = System::new();
    system.init_logger();

    let game_session =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/game_session.opt.wasm")
            .with_id(GAME_SESSION_PROGRAM_ID)
            .build(&system);

    let wordle =
        ProgramBuilder::from_file("../target/wasm32-unknown-unknown/debug/wordle.opt.wasm")
            .with_id(WORDLE_PROGRAM_ID)
            .build(&system);

    let result = wordle.send_bytes(USER, []);
    assert!(!result.main_failed());

    let result = game_session.send(
        USER,
        GameSessionInit {
            wordle_program_id: WORDLE_PROGRAM_ID.into(),
        },
    );
    assert!(!result.main_failed());

    // 第一轮游戏
    let result = game_session.send(USER, GameSessionAction::StartGame);
    assert!(!result.main_failed());

    let result = game_session.send(USER, GameSessionAction::CheckWord("horse".to_string()));
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::GameOver(GameStatus::Win));
    assert!(!result.main_failed() && result.contains(&log));

    // 重新开始游戏
    let result = game_session.send(USER, GameSessionAction::StartGame);
    let log = Log::builder()
        .dest(USER)
        .source(GAME_SESSION_PROGRAM_ID)
        .payload(GameSessionEvent::StartSuccess);
    assert!(!result.main_failed() && result.contains(&log));
}
