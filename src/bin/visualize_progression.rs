use std::env;
use std::io::{self, Read};
use std::process;

use reversi_path_finder::board::{CellCoord, PlayerColor};
use reversi_path_finder::game::INITIAL_BOARD;
use serde_json::json;
use serde_json::Value;

fn player_to_string(player: &PlayerColor) -> &'static str {
    match player {
        PlayerColor::Black => "Black",
        PlayerColor::White => "White",
    }
}

fn error_and_exit(error: &str, detail: Option<String>) -> ! {
    let payload = json!({
        "bin": "visualize_progression",
        "status": "error",
        "error": error,
        "detail": detail,
    });
    println!("{}", serde_json::to_string(&payload).unwrap());
    process::exit(1);
}

fn read_stdin_json() -> Value {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        error_and_exit("stdin_read_failed", None);
    }
    if input.trim().is_empty() {
        error_and_exit("stdin_empty", None);
    }
    serde_json::from_str(&input).unwrap_or_else(|e| {
        error_and_exit("invalid_json", Some(e.to_string()));
    })
}

fn json_get_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(|v| v.as_str())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let progression_str = if args.len() == 1 {
        let value = read_stdin_json();
        json_get_str(&value, "progression")
            .unwrap_or_else(|| error_and_exit("missing_progression", None))
            .to_string()
    } else if args.len() == 2 {
        args[1].clone()
    } else {
        let payload = json!({
            "bin": "visualize_progression",
            "status": "error",
            "error": "usage",
            "usage": format!("{} <progression>  (or JSON via stdin with progression)", args[0]),
        });
        println!("{}", serde_json::to_string(&payload).unwrap());
        process::exit(1);
    };

    let mut board = INITIAL_BOARD.clone();
    let mut current_player = PlayerColor::Black;
    let mut move_number = 0;
    let mut steps = Vec::new();

    for move_str in progression_str.as_bytes().chunks(2) {
        let column = move_str[0] - b'A';
        let row = move_str[1] - b'1';
        let cell = CellCoord::new(column, row);

        let moves_current = board.moves_available(&current_player);
        let skipped = moves_current.is_empty();
        let actual_player = if skipped {
            current_player.opponent()
        } else {
            current_player
        };

        move_number += 1;

        board = board
            .place_disk(column, row, &actual_player)
            .unwrap_or_else(|| {
                error_and_exit(
                    "illegal_move",
                    Some(format!(
                        "move {} by {}",
                        cell.to_string(),
                        player_to_string(&actual_player)
                    )),
                );
            });
        steps.push(json!({
            "move_number": move_number,
            "move": cell.to_string(),
            "player": player_to_string(&actual_player),
            "skipped": skipped,
            "board_ascii": board.to_string_block(),
        }));

        current_player = actual_player.opponent();
    }

    let payload = json!({
        "bin": "visualize_progression",
        "status": "ok",
        "progression": progression_str,
        "initial_board_ascii": INITIAL_BOARD.to_string_block(),
        "final_board_ascii": board.to_string_block(),
        "final_matches_target": null,
        "steps": steps,
    });
    println!("{}", serde_json::to_string(&payload).unwrap());
}
