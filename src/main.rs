mod coloring;
mod constants;
mod positions;
mod labels;

use rustai_abalone::game::{AbaloneGame, Coord, MarbleMove, Board, BELGIAN_DAISY, EMPTY_BOARD};
use rustai_abalone::player::MagisterLudi;
use std::collections::HashMap;
use eframe::egui;
use egui::{Sense, Shape, Vec2, Align2};
use epaint::{pos2, vec2, CircleShape, Color32, Pos2, Stroke};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::thread::JoinHandle;
use std::{thread, time};

use coloring::AbaloneColors;
use constants::{BASE_WIDTH, BASE_HEIGHT, MARBLE_SIZE, DIST_SIZE, VEC_LEN};
use positions::AbalonePositions;
use labels::AbaloneLabels;

fn load_image_from_path(path: &std::path::Path) -> Result<egui::ColorImage, image::ImageError> {
    let image = image::ImageReader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels.as_slice(),
    ))
}

enum GUIWindow {
    Start,
    Game,
    Options
}

struct AbaloneGUI {
    /// abalone game implementation
    game: AbaloneGame,
    current_window: GUIWindow,
    starting_positions: Vec<Board>,
    selected_index: usize,
    black_ai: bool,
    white_ai: bool,
    /// skull image for dead marbles
    skull_marble: egui::TextureHandle,
    /// black marble image
    black_marble: egui::TextureHandle,
    /// white marble image
    white_marble: egui::TextureHandle,
    /// all positions for gui
    pos: AbalonePositions,
    /// marble image for inactive button
    nomove_marble: egui::TextureHandle,
    /// maps the possble moves to a follow-up state
    move_states: HashMap<MarbleMove, Board>,
    /// all colors come here:
    colors: AbaloneColors,
    /// all egui stuff down here
    uv: egui::Rect,
    glabels: AbaloneLabels,
    worker: Option<JoinHandle<()>>,
    gui_sender: Sender<(Board, bool)>,
    gui_receiver: Receiver<(Board, bool)>,
    worker_sender: Sender<Board>,
    worker_receiver: Receiver<Board>,
}

impl AbaloneGUI {
    const ROW_LENGTHS: [f32; 9] = [5.0, 6.0, 7.0, 8.0, 9.0, 8.0, 7.0, 6.0, 5.0];
    const COL_OFFSETS: [usize; 9] = [5, 4, 3, 2, 1, 1, 1, 1, 1];
    const COL_VALUES: [f32; 9] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];

    pub fn new(cc: &eframe::CreationContext<'_>, board: Board, mut starting_positions: Vec<Board>) -> Self {
        let base_path = std::path::Path::new(r"src\images");
        let skull_path = base_path.join("skull.png");
        let black_path = base_path.join("marble_blue.png");
        let white_path = base_path.join("marble_yellow.png");
        let nomove_path = base_path.join("marble_empty.png");

        let (gtx, grx) = unbounded();
        let (wtx, wrx) = unbounded();
        if starting_positions.len() < 1 {
            starting_positions.push(BELGIAN_DAISY);
        }
        let mut gui = Self {
            game: AbaloneGame::new(board),
            current_window: GUIWindow::Game,
            starting_positions,
            selected_index: 0,
            black_ai: false,
            white_ai: false,
            skull_marble: cc.egui_ctx.load_texture(
                "skull",
                load_image_from_path(skull_path.as_path()).unwrap(),
                egui::TextureOptions::default()),
            black_marble: cc.egui_ctx.load_texture(
                "black",
                load_image_from_path(black_path.as_path()).unwrap(),
                egui::TextureOptions::default()),
            white_marble: cc.egui_ctx.load_texture(
                "white",
                load_image_from_path(white_path.as_path()).unwrap(),
                egui::TextureOptions::default()),
            nomove_marble: cc.egui_ctx.load_texture(
                "nomove",
                load_image_from_path(nomove_path.as_path()).unwrap(),
                egui::TextureOptions::default()),
            pos: AbalonePositions::default(),
            move_states: HashMap::with_capacity(6),
            colors: AbaloneColors::default(),
            uv: egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            glabels: AbaloneLabels::default(),
            worker: None,
            gui_sender: gtx,
            gui_receiver: grx,
            worker_sender: wtx,
            worker_receiver: wrx,
        };
        gui.game_painter_vectors();
        gui.worker_thread();
        gui
    }

    fn perform_move(&mut self, mut next_state: Board) {
        // clear possible moves
        self.move_states.clear();
        self.game.differences_to_state(next_state, &mut self.pos.color_selection);
        let was_black_last = self.game.get_black_tomove();
        // is this the best solution?
        let move_circles: Vec<Coord> = self.pos.color_selection.iter().map(|coord| coord.clone()).collect();
        
        // deselect / decolorize selected marbles after performing a move
        self.pos.color_selection.clear();
        self.pos.circle_selection.clear();

        // first update board -> actually perform move
        if was_black_last {
            next_state = AbaloneGame::rotate_board(next_state);
        }
        self.game.update_state(next_state);
        // should this be handled?
        let _ = self.gui_sender.send((next_state, self.game.get_game_ended()));

        // now adjust painter values
        self.game_painter_vectors();

        // add moved marbles to painter circles
        let color_fill = if was_black_last {&self.colors.color_black_fill} else {&self.colors.color_white_fill};
        let color_stroke = if was_black_last {&self.colors.move_black_stroke} else {&self.colors.move_white_stroke};
        for coord in move_circles {
            self.pos.circle_move_empty.push(
                Shape::Circle(CircleShape {
                    center: Self::coord_to_center(coord),
                    radius: 32.0,
                    fill: *color_fill,
                    stroke: Stroke::new(2.0, *color_stroke),
                })
            );
        }
    }

    fn game_painter_vectors(&mut self) {
        let (blacks, whites, empties) = self.game.get_coords_by_type();
        let (black_loss, white_loss) = self.game.get_black_white_loss();

        // handle board positions
        self.pos.black_pos = blacks.iter().map(|coord| (*coord, Self::coord_to_center(*coord))).collect();
        self.pos.white_pos = whites.iter().map(|coord| (*coord, Self::coord_to_center(*coord))).collect();
        self.pos.circle_move_empty.clear();
        for coord in empties {
            self.pos.circle_move_empty.push(
                Shape::Circle(CircleShape {
                    center: Self::coord_to_center(coord),
                    radius: 10.0,
                    fill: self.colors.color_empty_fill,
                    stroke: Stroke::new(5.0, self.colors.color_empty_stroke),
                })
            );
        }

        // handle dead zone positions
        self.pos.skull_pos.clear();
        let old_blackloss = self.pos.black_died.len();
        self.pos.black_died.clear();
        let old_whiteloss = self.pos.white_died.len();
        self.pos.white_died.clear();
        self.fill_deadzone_vectors(true, usize::from(black_loss), old_blackloss);
        self.fill_deadzone_vectors(false, usize::from(white_loss), old_whiteloss);

        // handle game message and color
        let result = self.game.get_game_result();
        let is_blacksmove = self.game.get_black_tomove();
        self.colors.text_color = match result {
            -1 => self.colors.color_black_stroke.clone(),
            0 => Color32::WHITE,
            1 => self.colors.color_white_stroke.clone(),
            _ => if is_blacksmove {self.colors.color_black_stroke.clone()} else {self.colors.color_white_stroke.clone()},
        };
        self.glabels.game_message = match result {
            -1 => format!("'{}' won the game!", self.glabels.black_name),
            0 => "The game ended in a draw".to_string(),
            1 => format!("'{}' won the game!", self.glabels.white_name),
            _ => if is_blacksmove {
                format!("'{}' has to make a move", self.glabels.black_name)
            } else {
                format!("'{}' has to make a move", self.glabels.white_name)
            },
        }

    }

    fn fill_deadzone_vectors(&mut self, is_black: bool, loss: usize, old_loss: usize) {
        let (deadzone, dead_marbles, color_stroke, color_fill) = if is_black {
            (&mut self.pos.black_deads, &mut self.pos.black_died, &self.colors.move_white_stroke, &self.colors.color_white_fill)
        } else {
            (&mut self.pos.white_deads, &mut self.pos.white_died, &self.colors.move_black_stroke, &self.colors.color_black_fill)
        };
        for (num, position) in deadzone.iter().enumerate() {
            if num < loss {
                dead_marbles.push(position.clone());
                self.pos.skull_pos.push(position.clone());
                if num == old_loss {
                    self.pos.circle_move_empty.push(
                        Shape::Circle(CircleShape {
                            center: *position,
                            radius: 32.0,
                            fill: *color_fill,
                            stroke: Stroke::new(2.0, *color_stroke),
                        })
                    );
                }
            } else {
                self.pos.circle_move_empty.push(
                    Shape::Circle(CircleShape {
                        center: *position,
                        radius: 10.0,
                        fill: self.colors.color_empty_fill,
                        stroke: Stroke::new(5.0, self.colors.color_empty_stroke),
                    })
                );
            }
        }
    }

    fn start_painer_vectors(&mut self) {
        let start_pos = self.starting_positions[self.selected_index];
        let (blacks, whites, empties) = AbaloneGame::coords_by_type(start_pos);

        // handle board positions
        self.pos.black_pos = blacks.iter().map(|coord| (*coord, Self::coord_to_center(*coord))).collect();
        self.pos.white_pos = whites.iter().map(|coord| (*coord, Self::coord_to_center(*coord))).collect();
        self.pos.circle_move_empty.clear();
        for coord in empties {
            self.pos.circle_move_empty.push(
                Shape::Circle(CircleShape {
                    center: Self::coord_to_center(coord),
                    radius: 10.0,
                    fill: self.colors.color_empty_fill,
                    stroke: Stroke::new(5.0, self.colors.color_empty_stroke),
                })
            );
        }
        self.glabels.game_message = "Choose starting position".to_string();
    }

    fn colorize_selection(&mut self, selec_coord: Coord) {
        if self.pos.color_selection.contains(&selec_coord) {
            // if the marble was already selected, de-select it
            self.pos.color_selection.remove(&selec_coord);
        } else if self.pos.allowed_selection.contains(&selec_coord) {
            // first adjust selection, there will be new selections no matter what happend before
            self.pos.allowed_selection.clear();
            if self.pos.color_selection.len() == 1 {
                // only special case, if the allowed_selection was empty, the condition would not be satisfied in the first place
                let init_coord = self.pos.color_selection.iter().next().unwrap();
                let move_diff = selec_coord - *init_coord;
                self.pos.allowed_selection.insert(selec_coord + move_diff);
                self.pos.allowed_selection.insert(*init_coord - move_diff);
            }

            // if the marble can be added to a line, add it
            self.pos.color_selection.insert(selec_coord);

        // adjust allowed selection
        } else {
            // otherwise this will be a new selection
            self.pos.color_selection.clear();
            self.pos.color_selection.insert(selec_coord);

            // now all positions around the selected one are allowed
            self.pos.allowed_selection.clear();
            for marb_move in AbaloneGame::get_game_moves() {
                self.pos.allowed_selection.insert(selec_coord + marb_move);
            }
        }

        // draw new circles
        let is_blacksmove = self.game.get_black_tomove();
        let color_fill = if is_blacksmove {&self.colors.color_black_fill} else {&self.colors.color_white_fill};
        let color_stroke = if is_blacksmove {&self.colors.color_black_stroke} else {&self.colors.color_white_stroke};
        self.pos.circle_selection = self.pos.color_selection.iter().map(|coord| {
            Shape::Circle(CircleShape {
                center: Self::coord_to_center(*coord),
                radius: 32.0,
                fill: *color_fill,
                stroke: Stroke::new(2.0, *color_stroke),
            })
        }).collect();

        // update buttons
        let start_coords = self.pos.color_selection.iter().map(|c| c.clone()).collect();
        self.move_states = self.game.calc_coord_moves(start_coords);
    }
    
    fn coord_to_center(coord: Coord) -> Pos2 {
        // note that in Coord x is for rows and y for columns
        // for the screen it is the other way round
        let x = BASE_WIDTH - DIST_SIZE * (
            0.5 * (Self::ROW_LENGTHS[coord.x-1] - 5.0) - Self::COL_VALUES[coord.y-Self::COL_OFFSETS[coord.x-1]]
        );
        let y = BASE_HEIGHT + MARBLE_SIZE * Self::COL_VALUES[coord.x-1];
        Pos2{ x, y }
    }

    fn worker_thread(&mut self) {
        // this only works from an initial position
        // chose parameters?
        let mut is_blacksmove = self.game.get_black_tomove();
        let mut black_magister = if self.black_ai {
            Some(MagisterLudi::new(self.game.get_state(), None, 40, 20, 7, 0))
        } else {None};
        let mut white_magister = if self.white_ai {
            Some(MagisterLudi::new(self.game.get_state(), None, 40, 20, 7, 0))
        } else {None};
        let g_recveiver = self.gui_receiver.clone();
        let w_sender = self.worker_sender.clone();
        let sleep_time = time::Duration::from_millis(10);
        self.worker = Some(thread::spawn(move || {
            // do this until finished
            'thread_loop: loop {
                // first the active player (if existant) will make a move
                let (current_player, waiting_player) = if is_blacksmove {
                    (&mut black_magister, &mut white_magister)
                } else {
                    (&mut white_magister, &mut black_magister)};
                match current_player {
                    Some(activegister) => {
                        let chosen_state = activegister.own_move(false);
                        let _ = w_sender.send(chosen_state);
                    }
                    None => {},
                }
                // wait for the GUI to adjust the move either from human hand or from the "active" AI who just sent a state
                'msg_loop: loop {
                    if let Ok((obatained_state, has_ended)) = g_recveiver.try_recv() {
                        match waiting_player {
                            Some(waitgister) => {
                                // state was adjusted
                                if has_ended {
                                    waitgister.stop_execution();
                                } else {
                                    waitgister.external_move(obatained_state, true);
                                }
                            }
                            _ => {}
                        }
                        // quit worker thread if game ended
                        if has_ended {
                            match current_player {
                                Some(activegister) => {
                                    activegister.stop_execution();
                                }
                                _ => {}
                            }
                            break 'thread_loop;
                        }
                        // quite receiving loop
                        break 'msg_loop;
                    } else {
                        // if there is no message, wait for it
                        thread::sleep(sleep_time);
                    }
                }
                // now a move was made and roles will be switched
                is_blacksmove = !is_blacksmove;
            }
        }));
    }

    fn start_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            
            let (_reponse, painter) = ui.allocate_painter(Vec2::new(1200.0, 700.0), Sense::hover());
            // print game message
            painter.text(
                pos2(BASE_WIDTH+200.0, BASE_HEIGHT/2.0),
                Align2::CENTER_CENTER,
                self.glabels.game_message.clone(),
                self.glabels.font.clone(),
                Color32::WHITE
            );
            painter.extend(self.pos.circle_move_empty.clone());

            let black_id = egui::TextureId::from(&self.black_marble);
            for (_, position) in self.pos.black_pos.iter() {
                painter.image(
                    black_id,
                    egui::Rect::from_center_size(*position, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                    self.uv,
                    Color32::WHITE,
                );
            }
            let white_id = egui::TextureId::from(&self.white_marble);
            for (_, position) in self.pos.white_pos.iter() {
                painter.image(
                    white_id,
                    egui::Rect::from_center_size(*position, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                    self.uv,
                    Color32::WHITE,
                );
            }

            // left and right switch button for position
            let button_id = egui::TextureId::from(&self.nomove_marble);
            let left_pos = Self::coord_to_center(Coord::new(6, 0));
            if ui.put(
                egui::Rect::from_center_size(
                    left_pos, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                egui::ImageButton::new(
                    (button_id, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)))
            ).clicked() {
                if self.selected_index == 0 {
                    self.selected_index = self.starting_positions.len() - 1;
                } else {
                    self.selected_index -= 1;
                }
                self.start_painer_vectors();
            };
            painter.arrow(left_pos, vec2(-VEC_LEN, 0.0), Stroke{width: 3.0, color: Color32::BLACK});
            let right_pos = Self::coord_to_center(Coord::new(6, 10));
            if ui.put(
                egui::Rect::from_center_size(
                    right_pos, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                egui::ImageButton::new(
                    (button_id, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)))
            ).clicked() {
                self.selected_index += 1;
                if self.selected_index == self.starting_positions.len() {
                    self.selected_index = 0;
                }
                self.start_painer_vectors();
            };
            painter.arrow(right_pos, vec2(VEC_LEN, 0.0), Stroke{width: 3.0, color: Color32::BLACK});
        });
    }

    fn game_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (_reponse, painter) = ui.allocate_painter(Vec2::new(1200.0, 700.0), Sense::hover());
            // paint circles first as the marbles will be printed above
            painter.extend(self.pos.circle_move_empty.clone());
            painter.extend(self.pos.circle_selection.clone());

            let is_blacksmove = self.game.get_black_tomove();
            let is_ended = self.game.get_game_ended();
            let skull_id = egui::TextureId::from(&self.skull_marble);
            let black_id = egui::TextureId::from(&self.black_marble);
            let white_id = egui::TextureId::from(&self.white_marble);
            let nomove_id = egui::TextureId::from(&self.nomove_marble);

            // print game message
            painter.text(
                pos2(BASE_WIDTH+200.0, BASE_HEIGHT),
                Align2::CENTER_CENTER,
                self.glabels.game_message.clone(),
                self.glabels.font.clone(),
                self.colors.text_color
            );

            // paint deadzones afterwards
            for position in self.pos.black_died.iter() {
                painter.image(
                    black_id,
                    egui::Rect::from_center_size(*position, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                    self.uv,
                    Color32::WHITE,
                );
                painter.image(
                    skull_id,
                    egui::Rect::from_center_size(*position, Vec2::new(42.0, 42.0)),
                    self.uv,
                    Color32::WHITE,
                );
            }
            for position in self.pos.white_died.iter() {
                painter.image(
                    white_id,
                    egui::Rect::from_center_size(*position, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                    self.uv,
                    Color32::WHITE,
                );
                painter.image(
                    skull_id,
                    egui::Rect::from_center_size(*position, Vec2::new(42.0, 42.0)),
                    self.uv,
                    Color32::WHITE,
                );
            }

            // paint clickable and unclickable marbles for active and waiting player respectively
            // later it has to be checked whether the active player is human or AI
            let (active_id, waiting_id, active_pos, waiting_pos) = if is_blacksmove {
                (black_id, white_id, &mut self.pos.black_pos, &mut self.pos.white_pos)
            } else {
                (white_id, black_id, &mut self.pos.white_pos, &mut self.pos.black_pos)
            };
            // easy part: just paint waiting player's marbles
            for (_, position) in waiting_pos.iter() {
                painter.image(
                    waiting_id,
                    egui::Rect::from_center_size(*position, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                    self.uv,
                    Color32::WHITE,
                );
            }
            // paint clickable images for color selection
            // check whether the current player is an AI-player or if the game has already ended
            let is_active = if is_blacksmove {!self.black_ai} else {!self.white_ai} && !is_ended;
            let mut selected_next_state: Option<Board> = None;
            let mut selected_coord: Option<Coord> = None;
            if is_active {
                // place buttons if the current player is human
                for (coord, position) in active_pos.iter() {
                    if ui.put(
                        egui::Rect::from_center_size(
                            *position, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                        egui::ImageButton::new(
                            (active_id, Vec2::new(MARBLE_SIZE, MARBLE_SIZE))).frame(false)
                    ).clicked() {
                       selected_coord = Some(*coord);
                    };
                }
            } else {
                // place images if the current player is AI
                for (_, position) in active_pos.iter() {
                    painter.image(
                        active_id,
                        egui::Rect::from_center_size(*position, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                        self.uv,
                        Color32::WHITE,
                    );
                }
                // receive next state from ai
                if let Ok(board_sent) = self.worker_receiver.try_recv() {
                    selected_next_state = Some(board_sent);
                }
            }

            // place buttons
            for (marb_move, position, direction) in self.pos.move_pos.iter() {
                match self.move_states.get(marb_move) {
                    Some(next_state) => {
                        if ui.put(
                            egui::Rect::from_center_size(
                                *position, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                            egui::ImageButton::new(
                                (active_id, Vec2::new(MARBLE_SIZE, MARBLE_SIZE))).frame(false)
                        ).clicked() {
                           selected_next_state = Some(*next_state);
                        };
                        painter.arrow(*position, *direction, Stroke{width: 3.0, color: Color32::WHITE});
                    }
                    _ => {
                        painter.image(
                            nomove_id,
                            egui::Rect::from_center_size(
                                *position, Vec2::new(MARBLE_SIZE, MARBLE_SIZE)),
                            self.uv,
                            Color32::WHITE,
                        );
                    }
                }
            }
            ui.end_row();
            // standard stuff up here

            let quit = self.add_another_button(ui, "Quit".to_string());
            if quit.clicked() {
                self.stop_worker();
                self.current_window = GUIWindow::Start;
            }
            ui.end_row();

            // handle clicks
            match selected_coord {
                Some(c) => self.colorize_selection(c),
                _ => {}
            };

            match selected_next_state {
                Some(n) => self.perform_move(n),
                _ => {}
            }
        });
    }

    fn option_window(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.add_exit_button(ui);
        });
    }

    fn add_exit_button(&self, ui: &mut egui::Ui) {
        if ui.add(
            egui::Button::new(
                egui::RichText::new("Exit").size(30.0)
            )
        ).clicked() {
            // game exit here
        }
    }

    fn add_another_button(&self, ui: &mut egui::Ui, button_text: String) -> egui::Response {
        ui.add(egui::Button::new(
                egui::RichText::new(button_text).size(30.0)
            )
        )
    }

    fn stop_worker(&mut self) {
        if let Some(handle) = self.worker.take(){
            let _ = self.gui_sender.send((EMPTY_BOARD, true));
            handle.join().unwrap();
        }
    }

}

impl eframe::App for AbaloneGUI {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        match self.current_window {
            GUIWindow::Start => self.start_window(ctx, frame),
            GUIWindow::Game => self.game_window(ctx, frame),
            GUIWindow::Options => self.option_window(ctx, frame),
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_worker();
    }
}

fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Play Abalone",
        native_options,
        Box::new(|cc| {
            let style = egui::Style {
                visuals: egui::Visuals::dark(),
                ..egui::Style::default()
            };
            cc.egui_ctx.set_style(style);
            Ok(Box::new(AbaloneGUI::new(cc, BELGIAN_DAISY, vec![])))
        }),
    );
}
