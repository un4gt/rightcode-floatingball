use iced::widget::canvas::{self, Cache, Canvas, Frame, Geometry, Path, Program, Stroke};
use iced::{Color, Element, Font, Point, Rectangle, Renderer, Size, Theme, mouse};

const FONT_CN: Font = Font::with_name("Microsoft YaHei");
const FONT_ICON: Font = Font::with_name("Segoe UI Symbol");

#[derive(Debug, Clone)]
pub enum BallEvent {
    StartDrag,
    ToggleSettings,
    RefreshNow,
    Scroll(i32),
    StartResize(Point),
    ResizeMove(Point),
    EndResize,
}

#[derive(Debug, Clone)]
pub enum BallStatus {
    Idle,
    Fetching,
    Error,
}

#[derive(Debug, Clone)]
pub struct BallDisplay {
    pub title: String,
    pub value: String,
    pub ratio: f32,
    pub status: BallStatus,
}

impl Default for BallDisplay {
    fn default() -> Self {
        Self {
            title: "未配置".to_string(),
            value: "--".to_string(),
            ratio: 0.0,
            status: BallStatus::Idle,
        }
    }
}

pub struct FloatingBall {
    base_cache: Cache,
    overlay_cache: Cache,
    display: BallDisplay,
    wave_phase: f32,
}

#[derive(Debug, Default)]
pub struct BallState {
    resizing: bool,
}

impl FloatingBall {
    pub fn new(display: BallDisplay) -> Self {
        Self {
            base_cache: Cache::new(),
            overlay_cache: Cache::new(),
            display,
            wave_phase: 0.0,
        }
    }

    pub fn set_display(&mut self, display: BallDisplay) {
        let overlay_changed = self.display.title != display.title
            || self.display.value != display.value
            || std::mem::discriminant(&self.display.status)
                != std::mem::discriminant(&display.status);

        if overlay_changed {
            self.overlay_cache.clear();
        }
        self.display = display;
    }

    pub fn set_wave_phase(&mut self, phase: f32) {
        self.wave_phase = phase;
    }

    pub fn view<'a, Message: 'a>(&'a self, size: f32) -> Element<'a, Message>
    where
        Message: From<BallEvent>,
    {
        Canvas::new(self)
            .width(iced::Length::Fixed(size))
            .height(iced::Length::Fixed(size))
            .into()
    }
}

impl<Message> Program<Message> for FloatingBall
where
    Message: From<BallEvent>,
{
    type State = BallState;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let base = self.base_cache.draw(renderer, bounds.size(), |frame| {
            draw_base(frame, bounds.size())
        });

        let mut water_frame = Frame::new(renderer, bounds.size());
        draw_water(
            &mut water_frame,
            bounds.size(),
            &self.display,
            self.wave_phase,
        );
        let water = water_frame.into_geometry();

        let overlay = self.overlay_cache.draw(renderer, bounds.size(), |frame| {
            draw_overlay(frame, bounds.size(), &self.display);
        });

        vec![base, water, overlay]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        let (center, radius) = circle_layout(bounds.size());
        let gear_hit = |position: Point| {
            let (gear_center, gear_radius) = gear_layout(center, radius);
            distance(position, gear_center) <= gear_radius
        };
        let resize_hit = |position: Point| {
            let (handle_center, handle_radius) = resize_layout(center, radius);
            distance(position, handle_center) <= handle_radius
        };

        match event {
            canvas::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)) => {
                let Some(position) = cursor.position_in(bounds) else {
                    return (canvas::event::Status::Ignored, None);
                };

                if distance(position, center) > radius {
                    return (canvas::event::Status::Ignored, None);
                }

                if gear_hit(position) {
                    return (
                        canvas::event::Status::Captured,
                        Some(Message::from(BallEvent::ToggleSettings)),
                    );
                }

                if resize_hit(position) {
                    state.resizing = true;

                    let absolute = cursor.position().unwrap_or_else(|| {
                        Point::new(bounds.x + position.x, bounds.y + position.y)
                    });

                    return (
                        canvas::event::Status::Captured,
                        Some(Message::from(BallEvent::StartResize(absolute))),
                    );
                }

                (
                    canvas::event::Status::Captured,
                    Some(Message::from(BallEvent::StartDrag)),
                )
            }
            canvas::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))
                if state.resizing =>
            {
                state.resizing = false;
                (
                    canvas::event::Status::Captured,
                    Some(Message::from(BallEvent::EndResize)),
                )
            }
            canvas::Event::Mouse(iced::mouse::Event::CursorMoved { position })
                if state.resizing =>
            {
                (
                    canvas::event::Status::Captured,
                    Some(Message::from(BallEvent::ResizeMove(position))),
                )
            }
            canvas::Event::Mouse(iced::mouse::Event::CursorLeft) if state.resizing => {
                state.resizing = false;
                (
                    canvas::event::Status::Captured,
                    Some(Message::from(BallEvent::EndResize)),
                )
            }
            canvas::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Right)) => {
                let Some(position) = cursor.position_in(bounds) else {
                    return (canvas::event::Status::Ignored, None);
                };

                if distance(position, center) > radius {
                    return (canvas::event::Status::Ignored, None);
                }

                (
                    canvas::event::Status::Captured,
                    Some(Message::from(BallEvent::RefreshNow)),
                )
            }
            canvas::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                let Some(position) = cursor.position_in(bounds) else {
                    return (canvas::event::Status::Ignored, None);
                };

                if distance(position, center) > radius {
                    return (canvas::event::Status::Ignored, None);
                }

                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y,
                    mouse::ScrollDelta::Pixels { y, .. } => y,
                };

                if y.abs() < f32::EPSILON {
                    return (canvas::event::Status::Ignored, None);
                }

                let steps = if y > 0.0 { -1 } else { 1 };

                (
                    canvas::event::Status::Captured,
                    Some(Message::from(BallEvent::Scroll(steps))),
                )
            }
            _ => (canvas::event::Status::Ignored, None),
        }
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.resizing {
            return mouse::Interaction::ResizingDiagonallyDown;
        }

        let Some(position) = cursor.position_in(bounds) else {
            return mouse::Interaction::None;
        };

        let (center, radius) = circle_layout(bounds.size());
        if distance(position, center) > radius {
            return mouse::Interaction::None;
        }

        let (handle_center, handle_radius) = resize_layout(center, radius);
        if distance(position, handle_center) <= handle_radius {
            return mouse::Interaction::ResizingDiagonallyDown;
        }

        mouse::Interaction::None
    }
}

fn draw_base(frame: &mut Frame, size: Size) {
    let (center, radius) = circle_layout(size);
    let circle = Path::circle(center, radius);

    // Matrix/极客风 - 深蓝黑渐变背景
    let background = canvas::gradient::Linear::new(
        Point::new(center.x - radius, center.y - radius),
        Point::new(center.x + radius, center.y + radius),
    )
    .add_stop(0.0, Color::from_rgba8(12, 25, 45, 245.0 / 255.0)) // 深蓝
    .add_stop(0.5, Color::from_rgba8(8, 18, 32, 250.0 / 255.0)) // 中间
    .add_stop(1.0, Color::from_rgba8(5, 12, 22, 255.0 / 255.0)); // 深黑

    frame.fill(&circle, background);
}

fn draw_water(frame: &mut Frame, size: Size, display: &BallDisplay, phase: f32) {
    let (center, radius) = circle_layout(size);
    let fill_ratio = display.ratio.clamp(0.0, 1.0);
    if fill_ratio <= 0.0 {
        return;
    }

    // Matrix/极客风 - 蓝绿色渐变水面
    let water_gradient = canvas::gradient::Linear::new(
        Point::new(center.x, center.y - radius),
        Point::new(center.x, center.y + radius),
    )
    .add_stop(0.0, Color::from_rgba8(0, 255, 200, 200.0 / 255.0)) // #00ffc8 青绿色
    .add_stop(0.5, Color::from_rgba8(0, 180, 255, 210.0 / 255.0)) // 中间过渡
    .add_stop(1.0, Color::from_rgba8(0, 136, 255, 220.0 / 255.0)); // #0088ff 蓝色

    if fill_ratio >= 1.0 {
        frame.fill(&Path::circle(center, radius), water_gradient);
        return;
    }

    let Some(water_path) = filled_wave_path(center, radius, fill_ratio, phase) else {
        return;
    };

    frame.fill(&water_path, water_gradient);
    // 添加深色叠加增加层次感
    frame.fill(&water_path, Color::from_rgba8(0, 30, 60, 25.0 / 255.0));

    if let Some(wave_line) = wave_surface_path(center, radius, fill_ratio, phase) {
        // 波浪高光 - 霓虹绿
        frame.stroke(
            &wave_line,
            Stroke::default()
                .with_width((radius * 0.035).max(1.6))
                .with_color(Color::from_rgba8(0, 255, 180, 140.0 / 255.0)),
        );
        // 波浪阴影线
        frame.stroke(
            &wave_line,
            Stroke::default()
                .with_width((radius * 0.02).max(1.0))
                .with_color(Color::from_rgba8(0, 60, 100, 100.0 / 255.0)),
        );
    }
}

fn draw_overlay(frame: &mut Frame, size: Size, display: &BallDisplay) {
    let (center, radius) = circle_layout(size);
    let circle = Path::circle(center, radius);

    // Matrix/极客风边框颜色
    let outline_color = match &display.status {
        BallStatus::Error => Color::from_rgb8(255, 60, 90), // 霓虹红
        BallStatus::Fetching => Color::from_rgb8(0, 200, 255), // 霓虹青
        BallStatus::Idle => Color::from_rgba8(0, 255, 136, 220.0 / 255.0), // 霓虹绿
    };

    // 发光边框
    frame.stroke(
        &circle,
        Stroke::default().with_width(2.5).with_color(outline_color),
    );

    draw_text(frame, center, radius, display);
    draw_gear(frame, center, radius);
    draw_resize_handle(frame, center, radius);
}

fn draw_text(frame: &mut Frame, center: Point, radius: f32, display: &BallDisplay) {
    use iced::widget::canvas::Text;

    // 科技感文字颜色 - 淡青绿色
    let title_color = Color::from_rgba8(200, 255, 240, 230.0 / 255.0);
    let value_color = Color::from_rgba8(0, 255, 200, 245.0 / 255.0); // 霓虹绿
    let small_color = Color::from_rgba8(100, 200, 180, 180.0 / 255.0);

    let shadow = Color::from_rgba8(0, 40, 60, 180.0 / 255.0);
    let shadow_offset = (radius * 0.03).max(1.0);

    let title_position = Point::new(center.x, center.y - radius * 0.18);
    frame.fill_text(Text {
        content: display.title.clone(),
        position: Point::new(
            title_position.x + shadow_offset,
            title_position.y + shadow_offset,
        ),
        color: shadow,
        size: iced::Pixels((radius * 0.26).max(10.0)),
        font: FONT_CN,
        horizontal_alignment: iced::alignment::Horizontal::Center,
        vertical_alignment: iced::alignment::Vertical::Center,
        ..Text::default()
    });
    frame.fill_text(Text {
        content: display.title.clone(),
        position: title_position,
        color: title_color,
        size: iced::Pixels((radius * 0.26).max(10.0)),
        font: FONT_CN,
        horizontal_alignment: iced::alignment::Horizontal::Center,
        vertical_alignment: iced::alignment::Vertical::Center,
        ..Text::default()
    });

    let value_position = Point::new(center.x, center.y + radius * 0.10);
    frame.fill_text(Text {
        content: display.value.clone(),
        position: Point::new(
            value_position.x + shadow_offset,
            value_position.y + shadow_offset,
        ),
        color: shadow,
        size: iced::Pixels((radius * 0.34).max(12.0)),
        font: FONT_CN,
        horizontal_alignment: iced::alignment::Horizontal::Center,
        vertical_alignment: iced::alignment::Vertical::Center,
        ..Text::default()
    });
    frame.fill_text(Text {
        content: display.value.clone(),
        position: value_position,
        color: value_color,
        size: iced::Pixels((radius * 0.34).max(12.0)),
        font: FONT_CN,
        horizontal_alignment: iced::alignment::Horizontal::Center,
        vertical_alignment: iced::alignment::Vertical::Center,
        ..Text::default()
    });

    let hint = "滚轮切换 · 右键刷新";
    frame.fill_text(Text {
        content: hint.to_string(),
        position: Point::new(center.x, center.y + radius * 0.42),
        color: small_color,
        size: iced::Pixels((radius * 0.18).max(9.0)),
        font: FONT_CN,
        horizontal_alignment: iced::alignment::Horizontal::Center,
        vertical_alignment: iced::alignment::Vertical::Center,
        ..Text::default()
    });
}

fn draw_gear(frame: &mut Frame, center: Point, radius: f32) {
    use iced::widget::canvas::Text;

    let (gear_center, gear_radius) = gear_layout(center, radius);
    let gear_circle = Path::circle(gear_center, gear_radius);

    // 科技感齿轮背景
    frame.fill(&gear_circle, Color::from_rgba8(5, 20, 35, 200.0 / 255.0));
    frame.stroke(
        &gear_circle,
        Stroke::default()
            .with_width(1.5)
            .with_color(Color::from_rgba8(0, 255, 136, 180.0 / 255.0)), // 霓虹绿边框
    );

    frame.fill_text(Text {
        content: "⚙".to_string(),
        position: gear_center,
        color: Color::from_rgba8(0, 255, 180, 230.0 / 255.0), // 霓虹青绿色
        size: iced::Pixels((gear_radius * 1.3).max(11.0)),
        font: FONT_ICON,
        horizontal_alignment: iced::alignment::Horizontal::Center,
        vertical_alignment: iced::alignment::Vertical::Center,
        ..Text::default()
    });
}

fn circle_layout(size: Size) -> (Point, f32) {
    // 使用接近 0.5 的比例，让圆填满窗口，避免缩放时露出边角
    let radius = (size.width.min(size.height) * 0.495).max(1.0);
    let center = Point::new(size.width / 2.0, size.height / 2.0);
    (center, radius)
}

fn gear_layout(center: Point, radius: f32) -> (Point, f32) {
    let gear_radius = radius * 0.22;
    let gear_center = Point::new(center.x + radius * 0.55, center.y - radius * 0.55);
    (gear_center, gear_radius)
}

fn resize_layout(center: Point, radius: f32) -> (Point, f32) {
    let handle_radius = radius * 0.22;
    let handle_center = Point::new(center.x + radius * 0.55, center.y + radius * 0.55);
    (handle_center, handle_radius)
}

fn draw_resize_handle(frame: &mut Frame, center: Point, radius: f32) {
    let (handle_center, handle_radius) = resize_layout(center, radius);
    let handle_circle = Path::circle(handle_center, handle_radius);

    // 科技感调整手柄
    frame.fill(&handle_circle, Color::from_rgba8(5, 20, 35, 180.0 / 255.0));
    frame.stroke(
        &handle_circle,
        Stroke::default()
            .with_width(1.5)
            .with_color(Color::from_rgba8(0, 200, 255, 150.0 / 255.0)), // 霓虹青边框
    );

    let grip_color = Color::from_rgba8(0, 255, 200, 180.0 / 255.0); // 霓虹绿
    let grip_stroke = Stroke::default()
        .with_width((handle_radius * 0.12).max(1.0))
        .with_color(grip_color);

    // 穿过圆心的45度斜线（从左上到右下）
    let line_len = handle_radius * 0.4;
    for i in 0..3 {
        let offset = (i as f32 - 1.0) * handle_radius * 0.22; // -1, 0, 1 偏移
        let from = Point::new(
            handle_center.x - line_len + offset,
            handle_center.y - line_len + offset,
        );
        let to = Point::new(
            handle_center.x + line_len + offset,
            handle_center.y + line_len + offset,
        );

        frame.stroke(&Path::line(from, to), grip_stroke);
    }
}

fn filled_wave_path(center: Point, radius: f32, ratio: f32, phase: f32) -> Option<Path> {
    if !(0.0..=1.0).contains(&ratio) {
        return None;
    }
    if ratio <= 0.0 {
        return None;
    }
    if ratio >= 1.0 {
        return Some(Path::circle(center, radius));
    }

    let segment = water_segment(center, radius, ratio)?;
    let left = segment.left;
    let right = segment.right;

    let wave_samples = 64;
    let arc_samples = 96;

    let width = (right.x - left.x).abs().max(1.0);
    let base_y = left.y;

    // 波浪强度随水位变化（中间最强，边缘最弱）
    let strength = (ratio * (1.0 - ratio) * 4.0).clamp(0.0, 1.0);

    // 整体摆动效果 - 模拟容器被晃动
    let sway_phase = phase * 0.6; // 摆动频率较慢
    let sway_amplitude = radius * 0.035 * strength; // 摆动幅度
    let sway_offset = sway_phase.sin() * sway_amplitude;

    // 波浪振幅
    let wave_amplitude = radius * 0.055 * strength;

    // 多频率波浪参数
    let k1 = std::f32::consts::TAU * 1.8 / width; // 主波
    let k2 = std::f32::consts::TAU * 3.5 / width; // 次波
    let k3 = std::f32::consts::TAU * 5.2 / width; // 细节波

    Some(Path::new(|builder| {
        builder.move_to(left);

        for i in 1..=wave_samples {
            let t = i as f32 / wave_samples as f32;
            let x = left.x + t * (right.x - left.x);

            // 边缘衰减
            let edge = (t * (1.0 - t) * 4.0).clamp(0.0, 1.0);

            let dx = x - left.x;

            // 多频率波浪叠加
            let wave1 = (k1 * dx + phase).sin() * 0.55; // 主波
            let wave2 = (k2 * dx - phase * 1.4).sin() * 0.30; // 次波（反向）
            let wave3 = (k3 * dx + phase * 2.1).sin() * 0.15; // 细节波
            let wobble = wave1 + wave2 + wave3;

            // 整体摆动：左侧和右侧相反方向移动
            let sway = sway_offset * (1.0 - 2.0 * t); // t=0时为+sway, t=1时为-sway

            let y = clamp_to_circle(
                center,
                radius,
                x,
                base_y + wave_amplitude * edge * wobble + sway,
            );
            builder.line_to(Point::new(x, y));
        }

        for i in 1..=arc_samples {
            let t = i as f32 / arc_samples as f32;
            let theta = segment.theta_right + t * (segment.theta_left - segment.theta_right);
            builder.line_to(point_on_circle(center, radius, theta));
        }

        builder.close();
    }))
}

fn wave_surface_path(center: Point, radius: f32, ratio: f32, phase: f32) -> Option<Path> {
    if ratio <= 0.0 || ratio >= 1.0 {
        return None;
    }

    let segment = water_segment(center, radius, ratio)?;
    let left = segment.left;
    let right = segment.right;

    let wave_samples = 64;
    let width = (right.x - left.x).abs().max(1.0);
    let base_y = left.y;

    // 波浪强度随水位变化
    let strength = (ratio * (1.0 - ratio) * 4.0).clamp(0.0, 1.0);

    // 整体摆动效果
    let sway_phase = phase * 0.6;
    let sway_amplitude = radius * 0.035 * strength;
    let sway_offset = sway_phase.sin() * sway_amplitude;

    // 波浪振幅
    let wave_amplitude = radius * 0.055 * strength;

    // 多频率波浪参数
    let k1 = std::f32::consts::TAU * 1.8 / width;
    let k2 = std::f32::consts::TAU * 3.5 / width;
    let k3 = std::f32::consts::TAU * 5.2 / width;

    Some(Path::new(|builder| {
        builder.move_to(left);

        for i in 1..=wave_samples {
            let t = i as f32 / wave_samples as f32;
            let x = left.x + t * (right.x - left.x);
            let edge = (t * (1.0 - t) * 4.0).clamp(0.0, 1.0);

            let dx = x - left.x;

            // 多频率波浪叠加
            let wave1 = (k1 * dx + phase).sin() * 0.55;
            let wave2 = (k2 * dx - phase * 1.4).sin() * 0.30;
            let wave3 = (k3 * dx + phase * 2.1).sin() * 0.15;
            let wobble = wave1 + wave2 + wave3;

            // 整体摆动
            let sway = sway_offset * (1.0 - 2.0 * t);

            let y = clamp_to_circle(
                center,
                radius,
                x,
                base_y + wave_amplitude * edge * wobble + sway,
            );
            builder.line_to(Point::new(x, y));
        }
    }))
}

#[derive(Debug, Clone, Copy)]
struct WaterSegment {
    left: Point,
    right: Point,
    theta_right: f32,
    theta_left: f32,
}

fn water_segment(center: Point, radius: f32, ratio: f32) -> Option<WaterSegment> {
    if ratio <= 0.0 || ratio >= 1.0 {
        return None;
    }

    let s = (1.0 - 2.0 * ratio).clamp(-1.0, 1.0);
    let theta_right = s.asin();
    let theta_left = std::f32::consts::PI - theta_right;

    Some(WaterSegment {
        left: point_on_circle(center, radius, theta_left),
        right: point_on_circle(center, radius, theta_right),
        theta_right,
        theta_left,
    })
}

fn clamp_to_circle(center: Point, radius: f32, x: f32, y: f32) -> f32 {
    let dx = x - center.x;
    let inside = radius * radius - dx * dx;
    if inside <= 0.0 {
        return y;
    }

    let dy = inside.sqrt();
    let min_y = center.y - dy;
    let max_y = center.y + dy;
    y.clamp(min_y, max_y)
}

fn point_on_circle(center: Point, radius: f32, theta: f32) -> Point {
    Point::new(
        center.x + radius * theta.cos(),
        center.y + radius * theta.sin(),
    )
}

fn distance(a: Point, b: Point) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}
