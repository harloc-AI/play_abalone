use eframe::egui::FontId;

pub struct AbaloneLabels {
    pub game_message: String,
    pub white_name: String,
    pub black_name: String,
    pub font: FontId,
}

impl Default for AbaloneLabels {
    fn default() -> Self {
        Self {
            game_message: "".to_string(),
            black_name: "Blue Player".to_string(),
            white_name: "Yellow Player".to_string(),
            font: FontId::proportional(30.0)
        }
    }
}