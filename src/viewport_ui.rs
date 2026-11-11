use crate::{
    geometry::Mesh,
    scene::Scene,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolMode {
    Orbit,
    Move,
    Rotate,
    Scale,
}

impl ToolMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Orbit => "ORBIT",
            Self::Move => "MOVE",
            Self::Rotate => "ROTATE",
            Self::Scale => "SCALE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAction {
    Undo,
    Projects,
    Redo,
    AddCube,
    Duplicate,
    Delete,
    SetTool(ToolMode),
    ToggleEdit,
    ToggleOutliner,
    SelectObject(usize),
    Ignore,
}

const TOOLBAR_HEIGHT: f32 = 68.0;
const FOOTER_HEIGHT: f32 = 48.0;
const BUTTON_TOP: f32 = 8.0;
const BUTTON_HEIGHT: f32 = 52.0;
const BUTTON_GAP: f32 = 4.0;
const PANEL_TOP: f32 = 76.0;
const PANEL_ROW_HEIGHT: f32 = 30.0;

fn panel_width(width: f32) -> f32 {
    (width * 0.38).min(220.0).max(128.0).min((width - 12.0).max(0.0))
}

fn button_width(width: f32) -> f32 {
    ((width - 16.0 - BUTTON_GAP * 7.0) / 8.0).max(28.0)
}

fn button_x(index: usize, width: f32) -> f32 {
    8.0 + index as f32 * (button_width(width) + BUTTON_GAP)
}

pub fn is_ui_position(
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    outliner_open: bool,
) -> bool {
    let width = width.max(1) as f32;
    let height = height.max(1) as f32;

    if y <= TOOLBAR_HEIGHT || y >= height - FOOTER_HEIGHT {
        return true;
    }

    if outliner_open
        && x >= width - panel_width(width) - 8.0
        && y >= PANEL_TOP
        && y < height - FOOTER_HEIGHT
    {
        return true;
    }

    false
}

pub fn hit_test(
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    scene: &Scene,
    outliner_open: bool,
) -> Option<UiAction> {
    let width_px = width.max(1) as f32;
    let height_px = height.max(1) as f32;

    if y <= TOOLBAR_HEIGHT {
        let labels = [
            UiAction::AddCube,
            UiAction::Duplicate,
            UiAction::Delete,
            UiAction::SetTool(ToolMode::Move),
            UiAction::SetTool(ToolMode::Rotate),
            UiAction::SetTool(ToolMode::Scale),
            UiAction::ToggleEdit,
            UiAction::ToggleOutliner,
        ];

        let button_width = button_width(width_px);
        for (index, action) in labels.into_iter().enumerate() {
            let left = button_x(index, width_px);
            if x >= left && x <= left + button_width
                && y >= BUTTON_TOP && y <= BUTTON_TOP + BUTTON_HEIGHT
            {
                return Some(action);
            }
        }

        return Some(UiAction::Ignore);
    }

    if y >= height_px - FOOTER_HEIGHT {
        return Some(if x < 72.0 { UiAction::Undo }
            else if x < 144.0 { UiAction::Redo }
            else if x < 256.0 { UiAction::Projects }
            else { UiAction::Ignore });
    }

    if outliner_open {
        let left = width_px - panel_width(width_px) - 8.0;
        if x >= left && y >= PANEL_TOP && y < height_px - FOOTER_HEIGHT {
            let row_top = PANEL_TOP + 38.0;
            if y >= row_top {
                let index = ((y - row_top) / PANEL_ROW_HEIGHT).floor() as usize;
                if index < scene.object_count() {
                    return Some(UiAction::SelectObject(index));
                }
            }
            return Some(UiAction::Ignore);
        }
    }

    None
}

pub fn build_vertices(
    width: u32,
    height: u32,
    scene: &Scene,
    tool: ToolMode,
    edit_mode: bool,
    outliner_open: bool,
    recovery_status: &str,
) -> Vec<f32> {
    let width_px = width.max(1) as f32;
    let height_px = height.max(1) as f32;
    let mut vertices = Vec::new();

    rect(
        &mut vertices,
        width_px,
        height_px,
        0.0,
        0.0,
        width_px,
        TOOLBAR_HEIGHT,
        [0.035, 0.045, 0.070],
    );

    let actions = [
        ("ADD", [0.12, 0.34, 0.48]),
        ("DUP", [0.13, 0.18, 0.27]),
        ("DEL", [0.42, 0.13, 0.16]),
        ("MOV", [0.17, 0.24, 0.34]),
        ("ROT", [0.17, 0.24, 0.34]),
        ("SCL", [0.17, 0.24, 0.34]),
        ("EDIT", [0.24, 0.18, 0.36]),
        ("LIST", [0.13, 0.18, 0.27]),
    ];

    let active_tool = match tool {
        ToolMode::Orbit => None,
        ToolMode::Move => Some(3),
        ToolMode::Rotate => Some(4),
        ToolMode::Scale => Some(5),
    };

    let bw = button_width(width_px);
    for (index, (label, base_color)) in actions.into_iter().enumerate() {
        let x = button_x(index, width_px);
        let mut color = base_color;

        if Some(index) == active_tool {
            color = [0.72, 0.34, 0.10];
        } else if index == 6 && edit_mode {
            color = [0.50, 0.27, 0.72];
        } else if index == 7 && outliner_open {
            color = [0.23, 0.34, 0.50];
        }

        rect(
            &mut vertices,
            width_px,
            height_px,
            x,
            BUTTON_TOP,
            bw,
            BUTTON_HEIGHT,
            color,
        );

        let scale = 1.05_f32;
        let text_width = label.len() as f32 * 4.0 * scale - scale;
        let text_x = x + (bw - text_width) * 0.5;
        text(
            &mut vertices,
            width_px,
            height_px,
            label,
            text_x,
            BUTTON_TOP + 34.0,
            scale,
            [0.94, 0.96, 1.0],
        );
    }

    if outliner_open {
        let panel_w = panel_width(width_px);
        let panel_x = width_px - panel_w - 8.0;
        let panel_h = (height_px - PANEL_TOP - FOOTER_HEIGHT).max(38.0);

        rect(
            &mut vertices,
            width_px,
            height_px,
            panel_x,
            PANEL_TOP,
            panel_w,
            panel_h,
            [0.055, 0.070, 0.100],
        );

        text(
            &mut vertices,
            width_px,
            height_px,
            "OUTLINER",
            panel_x + 10.0,
            PANEL_TOP + 22.0,
            1.25,
            [0.65, 0.73, 0.85],
        );

        rect(
            &mut vertices,
            width_px,
            height_px,
            panel_x + 8.0,
            PANEL_TOP + 29.0,
            panel_w - 16.0,
            1.0,
            [0.20, 0.25, 0.33],
        );

        let available_rows = ((panel_h - 38.0) / PANEL_ROW_HEIGHT).floor().max(0.0) as usize;
        for (index, object) in scene.objects().iter().take(available_rows).enumerate() {
            let row_y = PANEL_TOP + 38.0 + index as f32 * PANEL_ROW_HEIGHT;
            let active = scene.active_object_id() == Some(object.id());

            if active {
                rect(
                    &mut vertices,
                    width_px,
                    height_px,
                    panel_x + 5.0,
                    row_y + 2.0,
                    panel_w - 10.0,
                    PANEL_ROW_HEIGHT - 4.0,
                    [0.34, 0.20, 0.10],
                );
            }

            let mut name = object.name().to_uppercase();
            if name.chars().count() > 17 {
                name = name.chars().take(17).collect();
            }

            text(
                &mut vertices,
                width_px,
                height_px,
                &name,
                panel_x + 12.0,
                row_y + 19.0,
                1.0,
                if active {
                    [1.0, 0.82, 0.58]
                } else {
                    [0.72, 0.78, 0.87]
                },
            );
        }

        if scene.is_empty() {
            text(
                &mut vertices,
                width_px,
                height_px,
                "EMPTY SCENE",
                panel_x + 10.0,
                PANEL_TOP + 60.0,
                1.0,
                [0.55, 0.61, 0.70],
            );
        }
    }

    rect(
        &mut vertices,
        width_px,
        height_px,
        0.0,
        height_px - FOOTER_HEIGHT,
        width_px,
        FOOTER_HEIGHT,
        [0.025, 0.033, 0.052],
    );

    for (label, x) in [("UNDO", 12.0), ("REDO", 84.0), ("PROJECTS", 152.0)] {
        text(&mut vertices, width_px, height_px, label, x,
            height_px - 20.0, 2.0, [0.94, 0.96, 1.0]);
    }

    let active_name = scene.active_object()
        .map(|object| object.name().to_uppercase())
        .unwrap_or_else(|| "NO OBJECT".to_string());

    let mut status = format!(
        "{} | {}  |  {} OBJECTS  |  {}  |  {} MODE",
        recovery_status,
        active_name,
        scene.object_count(),
        tool.label(),
        if edit_mode { "EDIT" } else { "OBJECT" },
    );

    if status.chars().count() > 100 {
        status = status.chars().take(100).collect();
    }

    text(
        &mut vertices,
        width_px,
        height_px,
        &status,
        268.0,
        height_px - 9.0,
        1.0,
        [0.68, 0.75, 0.86],
    );

    vertices
}

pub(crate) fn rect(
    output: &mut Vec<f32>,
    width: f32,
    height: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: [f32; 3],
) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }

    let x0 = x / width * 2.0 - 1.0;
    let x1 = (x + w) / width * 2.0 - 1.0;
    let y0 = 1.0 - y / height * 2.0;
    let y1 = 1.0 - (y + h) / height * 2.0;

    triangle(output, [x0, y0], [x1, y0], [x1, y1], color);
    triangle(output, [x0, y0], [x1, y1], [x0, y1], color);
}

fn triangle(
    output: &mut Vec<f32>,
    a: [f32; 2],
    b: [f32; 2],
    c: [f32; 2],
    color: [f32; 3],
) {
    for point in [a, b, c] {
        output.extend_from_slice(&[
            point[0], point[1],
            color[0], color[1], color[2],
        ]);
    }
}

pub(crate) fn text(
    output: &mut Vec<f32>,
    width: f32,
    height: f32,
    value: &str,
    x: f32,
    baseline_y: f32,
    scale: f32,
    color: [f32; 3],
) {
    let mut cursor_x = x;
    for character in value.chars() {
        if character == ' ' {
            cursor_x += 3.8 * scale;
            continue;
        }

        let rows = glyph(character);
        for (row, bits) in rows.into_iter().enumerate() {
            for col in 0..3 {
                if bits & (1 << (2 - col)) != 0 {
                    rect(
                        output,
                        width,
                        height,
                        cursor_x + col as f32 * scale,
                        baseline_y - 5.0 * scale + row as f32 * scale,
                        scale * 0.88,
                        scale * 0.88,
                        color,
                    );
                }
            }
        }

        cursor_x += 4.0 * scale;
    }
}

fn glyph(character: char) -> [u8; 5] {
    match character.to_ascii_uppercase() {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b111, 0b111, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b111, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b110, 0b001, 0b010, 0b100, 0b111],
        '3' => [0b110, 0b001, 0b010, 0b001, 0b110],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b110, 0b001, 0b110],
        '6' => [0b011, 0b100, 0b110, 0b101, 0b010],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b010, 0b101, 0b010, 0b101, 0b010],
        '9' => [0b010, 0b101, 0b011, 0b001, 0b110],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '|' => [0b010, 0b010, 0b010, 0b010, 0b010],
        '_' => [0b000, 0b000, 0b000, 0b000, 0b111],
        _ => [0b000; 5],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_with_two_objects() -> Scene {
        let mut scene = Scene::new();
        let first = scene.add_object("Cube", Mesh::cube(2.0).unwrap()).unwrap();
        scene.add_object("Cube Copy", Mesh::cube(1.0).unwrap()).unwrap();
        scene.select(first).unwrap();
        scene
    }

    #[test]
    fn toolbar_buttons_have_individual_hit_targets() {
        let scene = scene_with_two_objects();
        let action = hit_test(1280, 720, 18.0, 30.0, &scene, true);
        assert_eq!(action, Some(UiAction::AddCube));
    }

    #[test]
    fn outliner_rows_select_objects_by_index() {
        let scene = scene_with_two_objects();
        let action = hit_test(1280, 720, 1120.0, 122.0, &scene, true);
        assert_eq!(action, Some(UiAction::SelectObject(0)));
    }

    #[test]
    fn generated_overlay_vertices_are_finite_and_not_empty() {
        let scene = scene_with_two_objects();
        let vertices = build_vertices(
            1280, 720, &scene, ToolMode::Move, false, true, "SAVED"
        );

        assert!(!vertices.is_empty());
        assert_eq!(vertices.len() % 5, 0);
        assert!(vertices.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn hidden_outliner_does_not_block_viewport_touches() {
        assert!(!is_ui_position(1280, 720, 1100.0, 300.0, false));
        assert!(is_ui_position(1280, 720, 20.0, 30.0, false));
    }
}
