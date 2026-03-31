#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use iced::task::Task;
use iced::time;
use iced::widget::{button, checkbox, column, container, row, text, text_input};
use iced::{Element, Length, Settings, Subscription, Theme, application};
use std::thread::sleep;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

const DEFAULT_CENTER_X: i32 = 0;
const DEFAULT_CENTER_Y: i32 = 0;
const DEFAULT_RADIUS: i32 = 270;
const DEFAULT_HOLD_LEFT_BUTTON: bool = true;
const DEFAULT_START_DELAY_MS: u64 = 1200;
const DEFAULT_CIRCLE_DURATION_MS: u64 = 5000;
const DEFAULT_TICK_MS: u64 = 2;
const DEFAULT_POLL_INTERVAL_MS: u64 = 100;

#[derive(Debug, Clone)]
enum Message {
    StartCircle,
    CircleFinished(Result<(), String>),
    ToggleMonitor,
    MonitorTick,
    CenterXChanged(String),
    CenterYChanged(String),
    RadiusChanged(String),
    StartDelayChanged(String),
    CircleDurationChanged(String),
    TickChanged(String),
    PollIntervalChanged(String),
    HoldLeftChanged(bool),
}

#[derive(Default)]
struct CircleApp {
    circle_running: bool,
    monitor_enabled: bool,
    cursor_text: String,
    status_text: String,
    center_x: String,
    center_y: String,
    radius: String,
    hold_left_button: bool,
    start_delay_ms: String,
    circle_duration_ms: String,
    tick_ms: String,
    poll_interval_ms: String,
}

#[derive(Debug, Clone)]
struct CircleConfig {
    center_x: i32,
    center_y: i32,
    radius: i32,
    hold_left_button: bool,
    start_delay_ms: u64,
    circle_duration_ms: u64,
    tick_ms: u64,
}

impl CircleConfig {
    fn from_app(app: &CircleApp) -> Result<Self, String> {
        let center_x = parse_i32("center_x", &app.center_x)?;
        let center_y = parse_i32("center_y", &app.center_y)?;
        let radius = parse_i32("radius", &app.radius)?;
        let start_delay_ms = parse_u64("start_delay_ms", &app.start_delay_ms)?;
        let circle_duration_ms = parse_u64("circle_duration_ms", &app.circle_duration_ms)?;
        let tick_ms = parse_u64("tick_ms", &app.tick_ms)?;

        if radius <= 0 {
            return Err("radius must be > 0".to_string());
        }
        if circle_duration_ms == 0 {
            return Err("circle_duration_ms must be > 0".to_string());
        }

        Ok(Self {
            center_x,
            center_y,
            radius,
            hold_left_button: app.hold_left_button,
            start_delay_ms,
            circle_duration_ms,
            tick_ms,
        })
    }
}

fn main() -> iced::Result {
    application("Circle GUI", update, view)
        .subscription(subscription)
        .theme(|_| Theme::Dark)
        .settings(Settings::default())
        .run_with(|| {
            (
                CircleApp {
                    cursor_text: "x=0, y=0".to_string(),
                    status_text: "idle".to_string(),
                    center_x: DEFAULT_CENTER_X.to_string(),
                    center_y: DEFAULT_CENTER_Y.to_string(),
                    radius: DEFAULT_RADIUS.to_string(),
                    hold_left_button: DEFAULT_HOLD_LEFT_BUTTON,
                    start_delay_ms: DEFAULT_START_DELAY_MS.to_string(),
                    circle_duration_ms: DEFAULT_CIRCLE_DURATION_MS.to_string(),
                    tick_ms: DEFAULT_TICK_MS.to_string(),
                    poll_interval_ms: DEFAULT_POLL_INTERVAL_MS.to_string(),
                    ..CircleApp::default()
                },
                Task::none(),
            )
        })
}

fn update(app: &mut CircleApp, message: Message) -> Task<Message> {
    match message {
        Message::StartCircle => {
            if app.circle_running {
                return Task::none();
            }

            let config = match CircleConfig::from_app(app) {
                Ok(config) => config,
                Err(err) => {
                    app.status_text = format!("invalid config: {err}");
                    return Task::none();
                }
            };

            app.circle_running = true;
            app.status_text = "drawing circle".to_string();

            Task::perform(
                async move { run_circle(config).map_err(|err| err.to_string()) },
                Message::CircleFinished,
            )
        }
        Message::CircleFinished(result) => {
            app.circle_running = false;
            app.status_text = match result {
                Ok(()) => "circle complete".to_string(),
                Err(err) => format!("circle failed: {err}"),
            };
            Task::none()
        }
        Message::ToggleMonitor => {
            app.monitor_enabled = !app.monitor_enabled;
            app.status_text = if app.monitor_enabled {
                "monitoring cursor".to_string()
            } else {
                "monitor stopped".to_string()
            };
            Task::none()
        }
        Message::MonitorTick => {
            if app.monitor_enabled {
                match current_cursor_position() {
                    Ok((x, y)) => {
                        app.cursor_text = format!("x={x}, y={y}");
                    }
                    Err(err) => {
                        app.status_text = format!("cursor read failed: {err}");
                        app.monitor_enabled = false;
                    }
                }
            }
            Task::none()
        }
        Message::CenterXChanged(value) => {
            app.center_x = value;
            Task::none()
        }
        Message::CenterYChanged(value) => {
            app.center_y = value;
            Task::none()
        }
        Message::RadiusChanged(value) => {
            app.radius = value;
            Task::none()
        }
        Message::StartDelayChanged(value) => {
            app.start_delay_ms = value;
            Task::none()
        }
        Message::CircleDurationChanged(value) => {
            app.circle_duration_ms = value;
            Task::none()
        }
        Message::TickChanged(value) => {
            app.tick_ms = value;
            Task::none()
        }
        Message::PollIntervalChanged(value) => {
            app.poll_interval_ms = value;
            Task::none()
        }
        Message::HoldLeftChanged(value) => {
            app.hold_left_button = value;
            Task::none()
        }
    }
}

fn subscription(app: &CircleApp) -> Subscription<Message> {
    if app.monitor_enabled {
        let poll_interval_ms = parse_u64("poll_interval_ms", &app.poll_interval_ms)
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_POLL_INTERVAL_MS);

        time::every(Duration::from_millis(poll_interval_ms)).map(|_| Message::MonitorTick)
    } else {
        Subscription::none()
    }
}

fn view(app: &CircleApp) -> Element<'_, Message> {
    let draw_label = if app.circle_running {
        "Drawing..."
    } else {
        "Draw Circle"
    };
    let monitor_label = if app.monitor_enabled {
        "Stop Monitor"
    } else {
        "Start Monitor"
    };

    let content = column![
        text("Mouse Tools").size(28),
        row![
            text("Center X").width(120),
            text_input("", &app.center_x)
                .on_input(Message::CenterXChanged)
                .width(120),
            text("Center Y").width(120),
            text_input("", &app.center_y)
                .on_input(Message::CenterYChanged)
                .width(120),
        ]
        .spacing(12),
        row![
            text("Radius").width(120),
            text_input("", &app.radius)
                .on_input(Message::RadiusChanged)
                .width(120),
            text("Start Delay (ms)").width(120),
            text_input("", &app.start_delay_ms)
                .on_input(Message::StartDelayChanged)
                .width(120),
        ]
        .spacing(12),
        row![
            text("Circle Duration (ms)").width(120),
            text_input("", &app.circle_duration_ms)
                .on_input(Message::CircleDurationChanged)
                .width(120),
            text("Tick (ms)").width(120),
            text_input("", &app.tick_ms)
                .on_input(Message::TickChanged)
                .width(120),
        ]
        .spacing(12),
        row![
            text("Poll Interval (ms)").width(120),
            text_input("", &app.poll_interval_ms)
                .on_input(Message::PollIntervalChanged)
                .width(120),
            checkbox("hold left button", app.hold_left_button).on_toggle(Message::HoldLeftChanged),
        ]
        .spacing(12),
        row![
            button(text(draw_label)).on_press(Message::StartCircle),
            button(text(monitor_label)).on_press(Message::ToggleMonitor),
        ]
        .spacing(12),
        text(format!("cursor: {}", app.cursor_text)),
        text(format!("status: {}", app.status_text)),
    ]
    .spacing(16)
    .padding(24);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn run_circle(config: CircleConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    sleep(Duration::from_millis(config.start_delay_ms));

    let (start_x, start_y) = point_on_circle(config.center_x, config.center_y, config.radius, 0.0);
    move_cursor_abs(start_x, start_y)?;
    sleep(Duration::from_millis(20));

    if config.hold_left_button {
        send_mouse_button(MOUSEEVENTF_LEFTDOWN)?;
    }

    let start = Instant::now();
    let duration = Duration::from_millis(config.circle_duration_ms);
    loop {
        let elapsed = start.elapsed();
        let t = (elapsed.as_secs_f64() / duration.as_secs_f64()).min(1.0);
        let (x, y) = point_on_circle(config.center_x, config.center_y, config.radius, t);
        move_cursor_abs(x, y)?;

        if t >= 1.0 {
            break;
        }

        sleep(Duration::from_millis(config.tick_ms));
    }

    if config.hold_left_button {
        send_mouse_button(MOUSEEVENTF_LEFTUP)?;
    }

    Ok(())
}

fn current_cursor_position() -> Result<(i32, i32), Box<dyn std::error::Error + Send + Sync>> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point)? };
    Ok((point.x, point.y))
}

fn point_on_circle(center_x: i32, center_y: i32, radius: i32, t: f64) -> (i32, i32) {
    let theta = t * std::f64::consts::TAU;
    let x = center_x as f64 + radius as f64 * theta.cos();
    let y = center_y as f64 + radius as f64 * theta.sin();
    (x.round() as i32, y.round() as i32)
}

fn parse_i32(name: &str, value: &str) -> Result<i32, String> {
    value
        .trim()
        .parse::<i32>()
        .map_err(|_| format!("{name} must be an i32"))
}

fn parse_u64(name: &str, value: &str) -> Result<u64, String> {
    value
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("{name} must be a u64"))
}

fn send_mouse_button(
    flags: MOUSE_EVENT_FLAGS,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    Ok(())
}

fn move_cursor_abs(x: i32, y: i32) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    unsafe { SetCursorPos(x, y)? };
    Ok(())
}
