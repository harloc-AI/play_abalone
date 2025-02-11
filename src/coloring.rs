use epaint::Color32;

pub struct AbaloneColors {
    pub color_empty_fill: Color32,
    pub color_empty_stroke: Color32,
    pub color_black_fill: Color32,
    pub color_black_stroke: Color32,
    pub move_black_stroke: Color32,
    pub color_white_fill: Color32,
    pub color_white_stroke: Color32,
    pub move_white_stroke: Color32,
    pub text_color: Color32,
}

impl Default for AbaloneColors {
    fn default() -> Self {
        Self {
            color_empty_fill: Color32::from_rgb(220, 220, 220),
            color_empty_stroke: Color32::from_rgb(255, 255, 255),
            color_black_fill: Color32::TRANSPARENT,
            color_black_stroke: Color32::from_rgb(117, 162, 216),
            move_black_stroke: Color32::from_rgb(158,190,228),
            color_white_fill: Color32::TRANSPARENT,
            color_white_stroke: Color32::from_rgb(247, 201, 11),
            move_white_stroke: Color32::from_rgb(249,217,84),
            text_color: Color32::WHITE,
        }
    }
}
