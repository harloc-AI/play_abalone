use std::collections::HashSet;
use rustai_abalone::game::{Coord, MarbleMove};
use epaint::{pos2, Pos2, vec2, Vec2};
use crate::constants::{BASE_WIDTH, BASE_HEIGHT, MARBLE_SIZE, DIST_SIZE, VEC_LEN};
use eframe::egui::Shape;

pub struct AbalonePositions {
    /// skull positions
    pub skull_pos: Vec<Pos2>,
    /// black marble positions
    pub black_pos: Vec<(Coord, Pos2)>,
    /// possible positions for dead black marbles
    pub black_deads: Vec<Pos2>,
    /// postions for actually dead black marbles
    pub black_died: Vec<Pos2>,
    /// white marble positions
    pub white_pos: Vec<(Coord, Pos2)>,
    /// possible position for dead white marbles
    pub white_deads: Vec<Pos2>,
    /// positiosn for actually dead white marbles
    pub white_died: Vec<Pos2>,
    /// coordinates for selected marbles
    pub color_selection: HashSet<Coord>,
    /// marbles allowed to be selected
    pub allowed_selection: HashSet<Coord>,
    /// vector containing selection circles
    pub circle_selection: Vec<Shape>,
    /// vector containing last move & empty field circles
    pub circle_move_empty: Vec<Shape>,
    /// vector containg the moves and the correspoding button positions
    pub move_pos:  Vec<(MarbleMove, Pos2, Vec2)>,
}

impl Default for AbalonePositions {
    fn default() -> Self {
        let first_x = 70.0;
        let first_y = BASE_HEIGHT + DIST_SIZE;
        let black_deads: Vec<Pos2> = vec![
            pos2(first_x, first_y),
            pos2(first_x+DIST_SIZE, first_y),
            pos2(first_x+2.0*DIST_SIZE, first_y),
            pos2(first_x+0.5*DIST_SIZE, first_y+MARBLE_SIZE),
            pos2(first_x+1.5*DIST_SIZE, first_y+MARBLE_SIZE),
            pos2(first_x+DIST_SIZE, first_y+2.0*MARBLE_SIZE)
        ];
        let first_y = BASE_HEIGHT + 7.0 * MARBLE_SIZE;
        let white_deads: Vec<Pos2> = vec![
            pos2(first_x, first_y+2.0*MARBLE_SIZE),
            pos2(first_x+DIST_SIZE, first_y+2.0*MARBLE_SIZE),
            pos2(first_x+2.0*DIST_SIZE, first_y+2.0*MARBLE_SIZE),
            pos2(first_x+0.5*DIST_SIZE, first_y+MARBLE_SIZE),
            pos2(first_x+1.5*DIST_SIZE, first_y+MARBLE_SIZE),
            pos2(first_x+DIST_SIZE, first_y)
        ];

        let first_x = 2.0 * BASE_WIDTH + 1.5 * DIST_SIZE;
        let first_y = BASE_HEIGHT + 6.5 * MARBLE_SIZE;
        // good enough
        let sin60: f32 = 0.866025;
        let button_positions: Vec<(MarbleMove, Pos2, Vec2)> = vec![
            (MarbleMove { dx: -1, dy: 0 }, pos2(first_x+0.5*DIST_SIZE, first_y), vec2(-0.5*VEC_LEN, -sin60*VEC_LEN)),
            (MarbleMove { dx: -1, dy: 1 }, pos2(first_x+1.5*DIST_SIZE, first_y), vec2(0.5*VEC_LEN, -sin60*VEC_LEN)),
            (MarbleMove { dx: 0, dy: -1 }, pos2(first_x, first_y+MARBLE_SIZE), vec2(-VEC_LEN, 0.0)),
            (MarbleMove { dx: 0, dy: 1 }, pos2(first_x+2.0*DIST_SIZE, first_y+MARBLE_SIZE), vec2(VEC_LEN, 0.0)),
            (MarbleMove { dx: 1, dy: -1 }, pos2(first_x+0.5*DIST_SIZE, first_y+2.0*MARBLE_SIZE), vec2(-0.5*VEC_LEN, sin60*VEC_LEN)),
            (MarbleMove { dx: 1, dy: 0 }, pos2(first_x+1.5*DIST_SIZE, first_y+2.0*MARBLE_SIZE), vec2(0.5*VEC_LEN, sin60*VEC_LEN))
        ];
        Self {
            skull_pos: Vec::with_capacity(12),
            black_deads,
            black_died: Vec::with_capacity(6),
            black_pos: Vec::new(),
            white_deads,
            white_died: Vec::with_capacity(6),
            white_pos: Vec::new(),
            color_selection: HashSet::with_capacity(6),
            allowed_selection: HashSet::with_capacity(6),
            circle_selection: Vec::with_capacity(6),
            circle_move_empty: Vec::with_capacity(60),
            move_pos: button_positions,
        }
    }
}