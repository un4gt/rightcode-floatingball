use std::time::{Duration, Instant, SystemTime};

use iced::widget::{
    button, column, container, row, scrollable, text, text_input, Column,
};
use iced::{
    mouse, window, Color, Element, Font, Length, Point, Size, Subscription, Task,
    Theme,
};

use crate::api::{
    default_subscription_index, fetch_subscriptions, remaining_ratio,
    Subscription as ApiSubscription,
};
use crate::ball::{BallDisplay, BallEvent, BallStatus, FloatingBall};
use crate::config::{
    is_configured, try_parse_refresh_seconds, AppConfig, ConfigStore,
};

const DEFAULT_BALL_SIZE: f32 = 120.0;
const MIN_BALL_SIZE: f32 = 80.0;
const MAX_BALL_SIZE: f32 = 220.0;
const SETTINGS_WIDTH: f32 = 420.0;
const SETTINGS_HEIGHT: f32 = 440.0;
const WAVE_SPEED: f32 = 2.2;
const WAVE_TICK_MS: u64 = 33;

#[derive(Debug, Clone)]
pub enum Message {
    Ball(BallEvent),
    Tick,
    Animate(Instant),
    ToggleSettings,
    WindowId(Option<window::Id>),
    DragWindow,
    TokenChanged(String),
    CookieChanged(String),
    UserAgentChanged(String),
    RefreshSecondsChanged(String),
    PreferredNameChanged(String),
    SavePressed,
    Saved(Result<(), String>),
    Fetched(Result<Vec<ApiSubscription>, String>),
}

impl From<BallEvent> for Message {
    fn from(value: BallEvent) -> Self {
        Self::Ball(value)
    }
}

pub struct State {
    window_id: Option<window::Id>,
    store: ConfigStore,
    config: AppConfig,
    token_input: String,
    cookie_input: String,
    user_agent_input: String,
    refresh_seconds_input: String,
    preferred_name_input: String,
    show_settings: bool,
    fetching: bool,
    last_updated: Option<SystemTime>,
    last_error: Option<String>,
    subscriptions: Vec<ApiSubscription>,
    selected_index: Option<usize>,
    ball_size: f32,
    resize_drag: Option<ResizeDrag>,
    wave_origin: Instant,
    ball: FloatingBall,
}

#[derive(Debug, Clone, Copy)]
struct ResizeDrag {
    start_cursor: Point,
    start_size: f32,
}

pub fn run() -> iced::Result {
    iced::application("RightCode Floating Ball", update, view)
        .theme(|_| Theme::Dark)
        .subscription(subscription)
        .style(|_state, theme| {
            let palette = theme.extended_palette();
            iced::application::Appearance {
                background_color: Color::TRANSPARENT,
                text_color: palette.background.base.text,
            }
        })
        .default_font(Font::with_name("Microsoft YaHei"))
        .window(window::Settings {
            size: Size::new(DEFAULT_BALL_SIZE, DEFAULT_BALL_SIZE),
            decorations: false,
            transparent: true,
            resizable: false,
            level: window::Level::AlwaysOnTop,
            ..window::Settings::default()
        })
        .run_with(|| {
            let store =
                ConfigStore::new().expect("config directory should be available");
            let config = store.load().unwrap_or_default();

            let mut state = State {
                window_id: None,
                token_input: config.bearer_token.clone(),
                cookie_input: config.cookie.clone(),
                user_agent_input: config.user_agent.clone(),
                refresh_seconds_input: config.refresh_seconds.to_string(),
                preferred_name_input: config.preferred_subscription_name.clone(),
                store,
                config,
                show_settings: false,
                fetching: false,
                last_updated: None,
                last_error: None,
                subscriptions: Vec::new(),
                selected_index: None,
                ball_size: DEFAULT_BALL_SIZE,
                resize_drag: None,
                wave_origin: Instant::now(),
                ball: FloatingBall::new(BallDisplay::default()),
            };

            state.sync_ball_display();

            let window_task = window::get_oldest().map(Message::WindowId);

            let refresh_task = if is_configured(&state.config) {
                refresh_now(&mut state)
            } else {
                Task::none()
            };

            let initial_task = Task::batch([window_task, refresh_task]);

            (state, initial_task)
        })
}

fn subscription(state: &State) -> Subscription<Message> {
    if state.show_settings {
        return Subscription::none();
    }

    Subscription::batch([
        iced::time::every(Duration::from_secs(state.config.refresh_seconds.max(5)))
            .map(|_| Message::Tick),
        iced::time::every(Duration::from_millis(WAVE_TICK_MS))
            .map(Message::Animate),
    ])
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Ball(event) => match event {
            BallEvent::StartDrag => state
                .window_id
                .map(window::drag)
                .unwrap_or_else(Task::none),
            BallEvent::ToggleSettings => toggle_settings(state),
            BallEvent::RefreshNow => refresh_now(state),
            BallEvent::Scroll(steps) => {
                scroll_subscriptions(state, steps);
                Task::none()
            }
            BallEvent::StartResize(start_cursor) => {
                state.resize_drag = Some(ResizeDrag {
                    start_cursor,
                    start_size: state.ball_size,
                });
                Task::none()
            }
            BallEvent::ResizeMove(cursor) => resize_ball(state, cursor),
            BallEvent::EndResize => {
                state.resize_drag = None;
                Task::none()
            }
        },
        Message::Tick => refresh_now(state),
        Message::Animate(now) => {
            let elapsed = now.duration_since(state.wave_origin).as_secs_f32();
            let phase =
                (elapsed * WAVE_SPEED).rem_euclid(std::f32::consts::TAU);
            state.ball.set_wave_phase(phase);
            Task::none()
        }
        Message::ToggleSettings => toggle_settings(state),
        Message::DragWindow => state
            .window_id
            .map(window::drag)
            .unwrap_or_else(Task::none),
        Message::WindowId(id) => {
            state.window_id = id;
            Task::none()
        }
        Message::TokenChanged(value) => {
            state.token_input = value;
            Task::none()
        }
        Message::CookieChanged(value) => {
            state.cookie_input = value;
            Task::none()
        }
        Message::UserAgentChanged(value) => {
            state.user_agent_input = value;
            Task::none()
        }
        Message::RefreshSecondsChanged(value) => {
            state.refresh_seconds_input = value;
            Task::none()
        }
        Message::PreferredNameChanged(value) => {
            state.preferred_name_input = value;
            Task::none()
        }
        Message::SavePressed => save_settings(state),
        Message::Saved(result) => {
            if let Err(err) = result {
                state.last_error = Some(err);
            } else {
                state.last_error = None;
            }
            state.sync_ball_display();
            Task::none()
        }
        Message::Fetched(result) => {
            state.fetching = false;
            match result {
                Ok(subscriptions) => {
                    let previous_selection = state
                        .selected_index
                        .and_then(|i| state.subscriptions.get(i))
                        .map(|s| s.name.clone());

                    state.subscriptions = subscriptions;

                    state.selected_index = previous_selection
                        .as_deref()
                        .and_then(|name| {
                            state.subscriptions.iter().position(|s| s.name == name)
                        })
                        .or_else(|| {
                            default_subscription_index(
                                &state.subscriptions,
                                &state.config.preferred_subscription_name,
                            )
                        });

                    state.last_error = None;
                    state.last_updated = Some(SystemTime::now());
                }
                Err(err) => {
                    state.last_error = Some(err);
                }
            }
            state.sync_ball_display();
            Task::none()
        }
    }
}

fn view(state: &State) -> Element<'_, Message> {
    if state.show_settings {
        return view_settings(state);
    }

    container(state.ball.view(state.ball_size))
        .width(Length::Fixed(state.ball_size))
        .height(Length::Fixed(state.ball_size))
        .into()
}

fn view_settings(state: &State) -> Element<'_, Message> {
    let header_row = row![
        text("设置").size(22),
        iced::widget::horizontal_space(),
        button("关闭").on_press(Message::ToggleSettings),
    ]
    .align_y(iced::Alignment::Center)
    .width(Length::Fill);

    let header = iced::widget::mouse_area(header_row)
        .on_press(Message::DragWindow)
        .interaction(mouse::Interaction::Grab);

    let path = text(format!("配置文件: {}", state.store.display_path()))
        .size(12)
        .color(Color::from_rgba8(255, 255, 255, 160.0 / 255.0));

    let token = text_input("Authorization token (Bearer ...)", &state.token_input)
        .on_input(Message::TokenChanged)
        .padding(8);

    let cookie = text_input("Cookie 或 cf_clearance 值", &state.cookie_input)
        .on_input(Message::CookieChanged)
        .padding(8);

    let user_agent =
        text_input("User-Agent（需与获取 cf_clearance 的浏览器一致）", &state.user_agent_input)
            .on_input(Message::UserAgentChanged)
            .padding(8);

    let refresh = text_input("刷新间隔(秒)", &state.refresh_seconds_input)
        .on_input(Message::RefreshSecondsChanged)
        .padding(8);

    let preferred = text_input("优先显示订阅名", &state.preferred_name_input)
        .on_input(Message::PreferredNameChanged)
        .padding(8);

    let mut actions = row![
        button("保存").on_press(Message::SavePressed),
        button("立即刷新").on_press(Message::Tick),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    if let Some(err) = &state.last_error {
        actions =
            actions.push(text(err).color(Color::from_rgb8(240, 100, 100)));
    }

    let body: Column<Message> =
        column![path, token, cookie, user_agent, refresh, preferred, actions]
            .spacing(10)
            .padding(12);

    let content: Column<Message> = column![header, scrollable(body).height(Length::Fill)]
        .spacing(10);

    container(content)
        .width(Length::Fixed(SETTINGS_WIDTH))
        .height(Length::Fixed(SETTINGS_HEIGHT))
        .style(container::rounded_box)
        .into()
}

fn toggle_settings(state: &mut State) -> Task<Message> {
    state.show_settings = !state.show_settings;
    state.resize_drag = None;

    let new_size = if state.show_settings {
        Size::new(SETTINGS_WIDTH, SETTINGS_HEIGHT)
    } else {
        Size::new(state.ball_size, state.ball_size)
    };

    state.sync_ball_display();
    state
        .window_id
        .map(|id| window::resize(id, new_size))
        .unwrap_or_else(Task::none)
}

fn save_settings(state: &mut State) -> Task<Message> {
    state.config.bearer_token = state.token_input.trim().to_string();
    state.config.cookie = state.cookie_input.trim().to_string();
    state.config.user_agent = if state.user_agent_input.trim().is_empty() {
        AppConfig::default().user_agent
    } else {
        state.user_agent_input.trim().to_string()
    };

    if let Some(seconds) = try_parse_refresh_seconds(&state.refresh_seconds_input) {
        state.config.refresh_seconds = seconds.max(5);
    }

    if !state.preferred_name_input.trim().is_empty() {
        state.config.preferred_subscription_name =
            state.preferred_name_input.trim().to_string();
    }

    state.sync_ball_display();

    let store = state.store.clone();
    let config = state.config.clone();

    Task::perform(
        async move { store.save(&config).map_err(|e| e.to_string()) },
        Message::Saved,
    )
}

fn refresh_now(state: &mut State) -> Task<Message> {
    if state.fetching || !is_configured(&state.config) {
        state.sync_ball_display();
        return Task::none();
    }

    state.fetching = true;
    state.sync_ball_display();

    let config = state.config.clone();

    Task::perform(
        async move {
            let response = fetch_subscriptions(&config)
                .await
                .map_err(|e| e.to_string())?;
            Ok(response.subscriptions)
        },
        Message::Fetched,
    )
}

fn scroll_subscriptions(state: &mut State, steps: i32) {
    if steps == 0 || state.subscriptions.is_empty() {
        return;
    }

    let len = state.subscriptions.len() as i32;
    let current = state.selected_index.unwrap_or(0) as i32;
    let next = (current + steps).rem_euclid(len) as usize;

    state.selected_index = Some(next);
    state.sync_ball_display();
}

fn resize_ball(state: &mut State, cursor: Point) -> Task<Message> {
    let Some(drag) = state.resize_drag else {
        return Task::none();
    };

    if state.show_settings {
        return Task::none();
    }

    let dx = cursor.x - drag.start_cursor.x;
    let dy = cursor.y - drag.start_cursor.y;
    let delta = (dx + dy) / 2.0;

    let new_size = (drag.start_size + delta).clamp(MIN_BALL_SIZE, MAX_BALL_SIZE);
    if (new_size - state.ball_size).abs() < 0.5 {
        return Task::none();
    }

    state.ball_size = new_size;
    state.sync_ball_display();

    state
        .window_id
        .map(|id| window::resize(id, Size::new(new_size, new_size)))
        .unwrap_or_else(Task::none)
}

impl State {
    fn sync_ball_display(&mut self) {
        let selected = self
            .selected_index
            .and_then(|i| self.subscriptions.get(i))
            .or_else(|| self.subscriptions.first());

        let (title, mut value, ratio) = match (
            selected,
            is_configured(&self.config),
        ) {
            (_, false) => ("未配置".to_string(), "点右上设置".to_string(), 0.0),
            (Some(sub), true) => {
                let ratio = remaining_ratio(sub);
                let value = format!("{:.2}", sub.remaining_quota);
                (sub.name.clone(), value, ratio)
            }
            (None, true) => ("无订阅".to_string(), "0.00".to_string(), 0.0),
        };

        if self.fetching {
            value = "...".to_string();
        }

        let status = if self.fetching {
            BallStatus::Fetching
        } else if self.last_error.is_some() {
            BallStatus::Error
        } else {
            BallStatus::Idle
        };

        self.ball.set_display(BallDisplay {
            title,
            value,
            ratio,
            status,
        });
    }
}
