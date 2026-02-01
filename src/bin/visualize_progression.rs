use std::env;
use std::io::{self, Read};
use std::process;

use reversi_path_finder::board::{Board, CellCoord, PlacementMask, PlayerColor};
use reversi_path_finder::game::INITIAL_BOARD;
use serde_json::Value;
use serde_json::json;

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

fn moves_available_with_mask(
    board: &Board,
    player: &PlayerColor,
    black_mask: &PlacementMask,
    white_mask: &PlacementMask,
) -> Vec<CellCoord> {
    let mask = match player {
        PlayerColor::Black => black_mask,
        PlayerColor::White => white_mask,
    };
    board
        .moves_available(player)
        .into_iter()
        .filter(|cell| mask.can_place_at_cell(*cell))
        .collect()
}

fn parse_from_json(
    value: &Value,
) -> (
    String,
    Option<Board>,
    Option<PlacementMask>,
    Option<PlacementMask>,
) {
    let progression = json_get_str(value, "progression")
        .unwrap_or_else(|| error_and_exit("missing_progression", None))
        .to_string();

    let input = value.get("input").unwrap_or(value);
    let white_board = json_get_str(input, "white_board_octal");
    let black_board = json_get_str(input, "black_board_octal");
    let black_mask = json_get_str(input, "black_mask_octal");
    let white_mask = json_get_str(input, "white_mask_octal");

    if let (Some(white_board), Some(black_board), Some(black_mask), Some(white_mask)) =
        (white_board, black_board, black_mask, white_mask)
    {
        let target_board = Board::from_octal_strings(white_board, black_board);
        let black_mask = PlacementMask::from_octal_string(black_mask);
        let white_mask = PlacementMask::from_octal_string(white_mask);
        (
            progression,
            Some(target_board),
            Some(black_mask),
            Some(white_mask),
        )
    } else {
        (progression, None, None, None)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let (progression_str, target_board, black_mask, white_mask) = if args.len() == 1 {
        let value = read_stdin_json();
        parse_from_json(&value)
    } else if args.len() == 2 {
        (args[1].clone(), None, None, None)
    } else {
        let payload = json!({
            "bin": "visualize_progression",
            "status": "error",
            "error": "usage",
            "usage": format!(
                "{} <progression>  (or JSON via stdin with progression and optional input)",
                args[0]
            ),
        });
        println!("{}", serde_json::to_string(&payload).unwrap());
        process::exit(1);
    };

    let mut board = INITIAL_BOARD.clone();
    let mut current_player = PlayerColor::Black;
    let mut move_number = 0;
    let mut steps = Vec::new();

    let bytes = progression_str.as_bytes();
    if bytes.len() % 2 != 0 {
        error_and_exit("invalid_progression_length", None);
    }

    for token in bytes.chunks(2) {
        let token_str = std::str::from_utf8(token).unwrap_or("--");

        if token_str == "--" {
            move_number += 1;
            let pass_player = current_player;
            steps.push(json!({
                "move_number": move_number,
                "move": null,
                "is_pass": true,
                "player": player_to_string(&pass_player),
                "skipped": true,
                "board_ascii": board.to_string_block(),
            }));
            current_player = pass_player.opponent();
            continue;
        }

        let column = token[0] - b'A';
        let row = token[1] - b'1';
        let cell = CellCoord::new(column, row);

        let moves_current = match (&black_mask, &white_mask) {
            (Some(black_mask), Some(white_mask)) => {
                moves_available_with_mask(&board, &current_player, black_mask, white_mask)
            }
            _ => board.moves_available(&current_player),
        };
        let skipped = moves_current.is_empty();
        let actual_player = if skipped {
            current_player.opponent()
        } else {
            current_player
        };

        move_number += 1;

        if !moves_current.is_empty()
            && !moves_current
                .iter()
                .any(|mv| *mv.column() == *cell.column() && *mv.row() == *cell.row())
        {
            error_and_exit(
                "illegal_move",
                Some(format!(
                    "move {} by {} not in available moves",
                    cell.to_string(),
                    player_to_string(&actual_player)
                )),
            );
        }

        if let (Some(black_mask), Some(white_mask)) = (&black_mask, &white_mask) {
            let mask_ok = match actual_player {
                PlayerColor::Black => black_mask.can_place_at_cell(cell),
                PlayerColor::White => white_mask.can_place_at_cell(cell),
            };
            if !mask_ok {
                error_and_exit(
                    "mask_violation",
                    Some(format!(
                        "move {} by {} not allowed by mask",
                        cell.to_string(),
                        player_to_string(&actual_player)
                    )),
                );
            }
        }

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
            "is_pass": false,
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
        "final_matches_target": target_board.as_ref().map(|t| *t == board),
        "steps": steps,
    });
    println!("{}", serde_json::to_string(&payload).unwrap());
}
