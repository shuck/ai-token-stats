use egui::{Align2, Color32, FontId, Rect, Sense, Stroke, Ui};

const FILL: Color32 = Color32::from_rgb(250, 252, 255);
const BORDER: Color32 = Color32::from_rgb(180, 198, 220);
const HOVER: Color32 = Color32::from_rgb(217, 236, 255);
const TEXT: Color32 = Color32::from_rgb(40, 50, 60);

pub fn label(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(10.0).color(TEXT));
}

pub fn button(ui: &mut Ui, text: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(58.0, 24.0), Sense::click());
    draw_box(ui, rect, response.hovered());
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(10.0),
        TEXT,
    );
    response.clicked()
}

pub fn combo(ui: &mut Ui, id: &str, selected: &str, items: &[&str], current: &mut usize) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(76.0, 24.0), Sense::click());
    let hovered = response.hovered();
    draw_box(ui, rect, hovered);
    let painter = ui.painter();
    painter.text(
        rect.left_center() + egui::vec2(8.0, 0.0),
        Align2::LEFT_CENTER,
        selected,
        FontId::proportional(10.0),
        TEXT,
    );
    painter.text(
        rect.right_center() + egui::vec2(-12.0, 0.0),
        Align2::CENTER_CENTER,
        "▾",
        FontId::proportional(10.0),
        Color32::from_rgb(90, 100, 115),
    );
    egui::popup_below_widget(
        ui,
        egui::Id::new(id),
        &response,
        |ui| {
            for (i, item) in items.iter().enumerate() {
                if ui
                    .selectable_label(*current == i, egui::RichText::new(*item).size(10.0))
                    .clicked()
                {
                    *current = i;
                    ui.close_menu();
                }
            }
        },
    );
}

fn draw_box(ui: &mut Ui, rect: Rect, hovered: bool) {
    let bg = if hovered { HOVER } else { FILL };
    ui.painter().rect_filled(rect, 6.0, bg);
    ui.painter()
        .rect_stroke(rect, 6.0, Stroke::new(1.0_f32, BORDER));
}
