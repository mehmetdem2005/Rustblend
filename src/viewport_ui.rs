use crate::scene::Scene;

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
    ResetTransform,
    SetTool(ToolMode),
    ToggleEdit,
    ToggleOutliner,
    SelectObject(usize),
    Ignore,
}

// All drawing and touch routing share logical coordinates. Scale with the surface,
// so a high-density phone does not get tiny fixed-pixel touch targets.
struct Layout {
    w: f32,
    h: f32,
    scale: f32,
    side: f32,
    split: f32,
}
impl Layout {
    fn new(width: u32, height: u32) -> Self {
        let scale =
            ((width.max(1) as f32 / 960.0).min(height.max(1) as f32 / 540.0)).clamp(0.5, 3.0);
        let w = width.max(1) as f32 / scale;
        let h = height.max(1) as f32 / scale;
        let side = (w * 0.27).clamp(180.0, 260.0).min((w - 160.0).max(0.0));
        Self {
            w,
            h,
            scale,
            side,
            split: (h * 0.51).max(220.0),
        }
    }
    fn rows(&self) -> usize {
        ((self.split - 132.0) / 32.0).floor().max(1.0) as usize
    }
    fn first(&self, scene: &Scene) -> usize {
        let active = scene
            .objects()
            .iter()
            .position(|o| Some(o.id()) == scene.active_object_id())
            .unwrap_or(0);
        active / self.rows() * self.rows()
    }
    fn inside(&self, x: f32, y: f32) -> bool {
        x.is_finite() && y.is_finite() && x >= 0.0 && y >= 0.0 && x < self.w && y < self.h
    }
}
const BG: [f32; 3] = [0.055, 0.058, 0.063];
const PANEL: [f32; 3] = [0.085, 0.089, 0.095];
const BUTTON: [f32; 3] = [0.13, 0.135, 0.145];
const ORANGE: [f32; 3] = [0.95, 0.36, 0.035];
const WHITE: [f32; 3] = [0.9, 0.91, 0.93];
const MUTED: [f32; 3] = [0.57, 0.60, 0.64];
const RAIL: [(&str, UiAction); 7] = [
    ("ORBIT", UiAction::SetTool(ToolMode::Orbit)),
    ("MOVE", UiAction::SetTool(ToolMode::Move)),
    ("ROTATE", UiAction::SetTool(ToolMode::Rotate)),
    ("SCALE", UiAction::SetTool(ToolMode::Scale)),
    ("ADD", UiAction::AddCube),
    ("COPY", UiAction::Duplicate),
    ("DELETE", UiAction::Delete),
];
pub fn is_ui_position(width: u32, height: u32, x: f32, y: f32, outliner_open: bool) -> bool {
    let l = Layout::new(width, height);
    let (x, y) = (x / l.scale, y / l.scale);
    l.inside(x, y)
        && (y < 88.0 || y >= l.h - 44.0 || x < 68.0 || (outliner_open && x >= l.w - l.side))
}
pub fn hit_test(
    width: u32,
    height: u32,
    x: f32,
    y: f32,
    scene: &Scene,
    outliner_open: bool,
) -> Option<UiAction> {
    let l = Layout::new(width, height);
    let (x, y) = (x / l.scale, y / l.scale);
    if !l.inside(x, y) {
        return None;
    }
    if y < 44.0 {
        return Some(if x >= l.w - 92.0 {
            UiAction::Projects
        } else {
            UiAction::Ignore
        });
    }
    if y < 88.0 {
        return Some(if x < 144.0 {
            UiAction::ToggleEdit
        } else if x >= l.w - 92.0 {
            UiAction::ToggleOutliner
        } else {
            UiAction::Ignore
        });
    }
    if y >= l.h - 44.0 {
        return Some(if x < 80.0 {
            UiAction::Undo
        } else if x < 160.0 {
            UiAction::Redo
        } else {
            UiAction::Ignore
        });
    }
    if x < 68.0 {
        let i = ((y - 92.0) / 52.0).floor() as usize;
        return Some(
            if y >= 92.0 && i < RAIL.len() && y < 92.0 + i as f32 * 52.0 + 48.0 {
                RAIL[i].1
            } else {
                UiAction::Ignore
            },
        );
    }
    if outliner_open && x >= l.w - l.side {
        if y >= 88.0 && y < 132.0 {
            let first = l.first(scene);
            let rows = l.rows();
            let index = if x < l.w - l.side / 2.0 {
                first.saturating_sub(rows)
            } else {
                first + rows
            };
            return Some(if index < scene.object_count() {
                UiAction::SelectObject(index)
            } else {
                UiAction::Ignore
            });
        }
        if y >= 132.0 && y < 132.0 + l.rows() as f32 * 32.0 {
            let i = l.first(scene) + ((y - 132.0) / 32.0).floor() as usize;
            return Some(if i < scene.object_count() {
                UiAction::SelectObject(i)
            } else {
                UiAction::Ignore
            });
        }
        if y >= l.split + 170.0 && y < l.split + 210.0 && y < l.h - 44.0 {
            return Some(UiAction::ResetTransform);
        }
        return Some(UiAction::Ignore);
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
    let l = Layout::new(width, height);
    let (w, h) = (l.w, l.h);
    let mut v = Vec::new();
    rect(&mut v, w, h, 0.0, 0.0, w, 88.0, BG);
    text(&mut v, w, h, "RUST", 16.0, 29.0, 3.0, WHITE);
    text(&mut v, w, h, "BLEND", 64.0, 29.0, 3.0, ORANGE);
    if w > 600.0 {
        text(&mut v, w, h, "MODELING WORKSPACE", 170.0, 27.0, 1.8, MUTED);
    }
    button(&mut v, &l, w - 92.0, 4.0, 88.0, 36.0, "PROJECTS", false);
    button(
        &mut v,
        &l,
        8.0,
        48.0,
        132.0,
        36.0,
        if edit_mode {
            "MESH PREVIEW"
        } else {
            "OBJECT MODE"
        },
        edit_mode,
    );
    text(&mut v, w, h, "PERSPECTIVE", 156.0, 71.0, 1.6, MUTED);
    button(
        &mut v,
        &l,
        w - 92.0,
        48.0,
        88.0,
        36.0,
        "PANELS",
        outliner_open,
    );
    rect(&mut v, w, h, 0.0, 88.0, 68.0, h - 132.0, BG);
    for (i, (label, action)) in RAIL.iter().enumerate() {
        let y = 92.0 + i as f32 * 52.0;
        if y + 48.0 <= h - 44.0 {
            button(
                &mut v,
                &l,
                4.0,
                y,
                60.0,
                48.0,
                label,
                *action == UiAction::SetTool(tool),
            );
        }
    }
    let viewport_right = if outliner_open { w - l.side } else { w };
    if viewport_right > 300.0 {
        text(
            &mut v,
            w,
            h,
            &format!("{} OBJECTS", scene.object_count()),
            82.0,
            110.0,
            1.6,
            MUTED,
        );
        text(&mut v, w, h, tool.label(), 82.0, 130.0, 1.6, ORANGE);
    }
    if outliner_open {
        let x = w - l.side;
        rect(&mut v, w, h, x, 88.0, l.side, h - 132.0, PANEL);
        button(
            &mut v,
            &l,
            x + 4.0,
            92.0,
            l.side / 2.0 - 8.0,
            32.0,
            "PREV",
            false,
        );
        button(
            &mut v,
            &l,
            x + l.side / 2.0,
            92.0,
            l.side / 2.0 - 4.0,
            32.0,
            "NEXT",
            false,
        );
        for (i, o) in scene
            .objects()
            .iter()
            .enumerate()
            .skip(l.first(scene))
            .take(l.rows())
        {
            let y = 132.0 + (i - l.first(scene)) as f32 * 32.0;
            let active = Some(o.id()) == scene.active_object_id();
            let name: String = o
                .name()
                .chars()
                .take(((l.side - 22.0) / 6.4) as usize)
                .collect();
            button(&mut v, &l, x + 4.0, y, l.side - 8.0, 28.0, &name, active);
        }
        if scene.is_empty() {
            text(&mut v, w, h, "EMPTY SCENE", x + 12.0, 154.0, 1.8, MUTED);
        }
        if l.split + 210.0 <= h - 44.0 {
            rect(&mut v, w, h, x, l.split, l.side, 2.0, BUTTON);
            text(
                &mut v,
                w,
                h,
                "TRANSFORM",
                x + 12.0,
                l.split + 26.0,
                2.0,
                WHITE,
            );
            if let Some(o) = scene.active_object() {
                let t = o.transform();
                let p = t.position();
                let s = t.scale();
                let q = t.rotation();
                for (i, line) in [
                    format!("X {:.2} Y {:.2}", p.x, p.y),
                    format!("Z {:.2}", p.z),
                    format!("S {:.2} {:.2} {:.2}", s.x, s.y, s.z),
                    format!("Q {:.2} {:.2}", q.x, q.y),
                    format!("  {:.2} {:.2}", q.z, q.w),
                ]
                .iter()
                .enumerate()
                {
                    let clipped: String = line
                        .chars()
                        .take(((l.side - 24.0) / 6.4) as usize)
                        .collect();
                    text(
                        &mut v,
                        w,
                        h,
                        &clipped,
                        x + 12.0,
                        l.split + 52.0 + i as f32 * 22.0,
                        1.6,
                        MUTED,
                    );
                }
                button(
                    &mut v,
                    &l,
                    x + 8.0,
                    l.split + 170.0,
                    l.side - 16.0,
                    40.0,
                    "RESET TRANSFORM",
                    false,
                );
            } else {
                text(
                    &mut v,
                    w,
                    h,
                    "NO SELECTION",
                    x + 12.0,
                    l.split + 54.0,
                    1.8,
                    MUTED,
                );
            }
        }
    }
    rect(&mut v, w, h, 0.0, h - 44.0, w, 44.0, BG);
    button(&mut v, &l, 4.0, h - 40.0, 72.0, 36.0, "UNDO", false);
    button(&mut v, &l, 84.0, h - 40.0, 72.0, 36.0, "REDO", false);
    let max = ((w - 178.0) / 6.4).max(0.0) as usize;
    let status: String = format!("{} | DRAG {} | PINCH ZOOM", recovery_status, tool.label())
        .chars()
        .take(max)
        .collect();
    text(&mut v, w, h, &status, 172.0, h - 17.0, 1.6, MUTED);
    v
}
fn button(v: &mut Vec<f32>, l: &Layout, x: f32, y: f32, w: f32, h: f32, label: &str, active: bool) {
    rect(
        v,
        l.w,
        l.h,
        x,
        y,
        w,
        h,
        if active { ORANGE } else { BUTTON },
    );
    let scale = 1.6;
    let tw = label.chars().count() as f32 * 4.0 * scale;
    text(
        v,
        l.w,
        l.h,
        label,
        x + (w - tw).max(8.0) / 2.0,
        y + h / 2.0 + 4.0,
        scale,
        WHITE,
    );
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

fn triangle(output: &mut Vec<f32>, a: [f32; 2], b: [f32; 2], c: [f32; 2], color: [f32; 3]) {
    for point in [a, b, c] {
        output.extend_from_slice(&[point[0], point[1], color[0], color[1], color[2]]);
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
    fn scene(n: usize) -> Scene {
        let mut s = Scene::new();
        for _ in 0..n {
            let id = s
                .add_object("Cube", crate::Mesh::cube(2.0).unwrap())
                .unwrap();
            s.select(id).unwrap();
        }
        s
    }
    #[test]
    fn ui_scales_hits_with_density() {
        let s = scene(1);
        for factor in [1, 2, 3] {
            assert_eq!(
                hit_test(
                    960 * factor,
                    540 * factor,
                    30.0 * factor as f32,
                    170.0 * factor as f32,
                    &s,
                    true
                ),
                Some(UiAction::SetTool(ToolMode::Move))
            );
        }
    }
    #[test]
    fn viewport_is_not_intercepted() {
        assert!(!is_ui_position(960, 540, 400.0, 250.0, true));
        assert!(is_ui_position(960, 540, 30.0, 250.0, false));
        assert!(!is_ui_position(960, 540, 900.0, 250.0, false));
    }
    #[test]
    fn paged_outliner_can_reach_last_object() {
        let s = scene(30);
        let l = Layout::new(960, 540);
        let first = l.first(&s);
        assert!(first > 0);
        assert_eq!(
            hit_test(960, 540, 850.0, 140.0, &s, true),
            Some(UiAction::SelectObject(first))
        );
        assert_eq!(
            hit_test(960, 540, 740.0, 104.0, &s, true),
            Some(UiAction::SelectObject(first - l.rows()))
        );
    }
    #[test]
    fn invalid_touch_is_ignored() {
        let s = scene(1);
        for (x, y) in [(f32::NAN, 20.0), (-1.0, 1.0), (30.0, f32::INFINITY)] {
            assert_eq!(hit_test(960, 540, x, y, &s, true), None);
        }
    }
    #[test]
    fn overlay_is_finite_on_phone_and_tablet() {
        for (w, h) in [(960, 540), (2340, 1080), (1080, 2340), (1, 1)] {
            let v = build_vertices(w, h, &scene(2), ToolMode::Move, false, true, "SAVED");
            assert!(!v.is_empty());
            assert_eq!(v.len() % 5, 0);
            assert!(v.iter().all(|x| x.is_finite()));
        }
    }
    #[test]
    fn inspector_reset_has_own_hit_target() {
        let l = Layout::new(960, 540);
        assert_eq!(
            hit_test(960, 540, 850.0, l.split + 180.0, &scene(1), true),
            Some(UiAction::ResetTransform)
        );
    }
}
