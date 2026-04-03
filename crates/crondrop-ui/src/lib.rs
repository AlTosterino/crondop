use anyhow::{Context, Result};
use crondrop_core::{AppConfig, ReminderAction};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row, text};
use iced::{Border, Color, Element, Length, Task, Theme, application, window};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::io::Cursor;
use std::time::Duration;

pub fn show_popup(config: AppConfig, reminder_id: String) -> Result<()> {
    configure_macos_popup_app();

    if config.behavior.sound {
        play_drop_sound_async();
    }

    application("Cron Drop", update, view)
        .theme(app_theme)
        .window(window::Settings {
            size: iced::Size::new(560.0, 360.0),
            resizable: false,
            icon: Some(build_window_icon()?),
            level: if config.ui.always_on_top {
                window::Level::AlwaysOnTop
            } else {
                window::Level::Normal
            },
            ..window::Settings::default()
        })
        .centered()
        .run_with(|| (PopupApp::from_config(config, reminder_id), Task::none()))?;

    Ok(())
}

#[cfg(target_os = "macos")]
fn configure_macos_popup_app() {
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSRunningApplication};
    use objc2_foundation::MainThreadMarker;

    if let Some(marker) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(marker);
        let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        app.hide(None);

        unsafe {
            let current_app = NSRunningApplication::currentApplication();
            let _ = current_app.hide();
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn configure_macos_popup_app() {}

#[derive(Debug, Clone)]
enum Message {
    Done,
    Snooze,
    Skip,
    PauseToday,
}

struct PopupApp {
    reminder_id: String,
    title: String,
    body: String,
    snooze_label: String,
}

impl PopupApp {
    fn from_config(config: AppConfig, reminder_id: String) -> Self {
        Self {
            reminder_id,
            title: config.popup.title,
            body: config.popup.body,
            snooze_label: format!("Snooze {}m", config.popup.snooze_minutes),
        }
    }
}

fn update(app: &mut PopupApp, message: Message) {
    let action = match message {
        Message::Done => ReminderAction::Done,
        Message::Snooze => ReminderAction::Snooze,
        Message::Skip => ReminderAction::Skip,
        Message::PauseToday => ReminderAction::PauseToday,
    };

    let result = crondrop_daemon::send_popup_action(app.reminder_id.clone(), action);
    if let Err(error) = result {
        eprintln!("failed to send popup action: {error}");
        std::process::exit(1);
    }

    std::process::exit(0);
}

fn view(app: &PopupApp) -> Element<'_, Message> {
    let brand = row![
        brand_dot(10.0, Color::from_rgb8(108, 142, 214)),
        text("Cron Drop")
            .size(16)
            .color(Color::from_rgb8(124, 108, 96)),
    ]
    .spacing(10)
    .align_y(Vertical::Center);

    let title = text(&app.title)
        .size(32)
        .color(Color::from_rgb8(56, 46, 39));

    let body = text(&app.body)
        .size(19)
        .line_height(1.35)
        .color(Color::from_rgb8(124, 109, 97));

    let hint = container(
        row![
            brand_dot(8.0, Color::from_rgb8(214, 178, 137)),
            text("A gentle nudge for your next drop")
                .size(14)
                .color(Color::from_rgb8(128, 110, 96)),
        ]
        .spacing(10)
        .align_y(Vertical::Center),
    )
    .padding([10, 14])
    .style(|_theme: &Theme| soft_pill_style());

    let primary = primary_button("Done", Message::Done).width(Length::FillPortion(3));
    let snooze = secondary_button(&app.snooze_label, Message::Snooze).width(Length::FillPortion(2));
    let skip = plain_action_button("Skip", Message::Skip);
    let pause = plain_action_button("Pause today", Message::PauseToday);

    let main_actions = row![primary, snooze].spacing(12);
    let quiet_actions = row![
        skip,
        text("•").size(14).color(Color::from_rgb8(190, 175, 160)),
        pause
    ]
    .spacing(10)
    .align_y(Vertical::Center);

    let footer = text("You can snooze if now is a bad moment.")
        .size(14)
        .color(Color::from_rgb8(155, 138, 124));

    let content = column![
        brand,
        title,
        body,
        hint,
        main_actions,
        quiet_actions,
        footer
    ]
    .spacing(18)
    .width(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([30, 34])
        .style(|_theme: &Theme| popup_style())
        .into()
}

fn app_theme(_app: &PopupApp) -> Theme {
    Theme::Light
}

fn popup_style() -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(253, 248, 243))
        .border(Border {
            radius: 28.0.into(),
            width: 1.0,
            color: Color::from_rgba8(214, 198, 184, 0.26),
        })
}

fn soft_pill_style() -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(249, 243, 236))
        .border(Border {
            radius: 999.0.into(),
            width: 1.0,
            color: Color::from_rgba8(205, 187, 169, 0.28),
        })
}

fn brand_dot(size: f32, color: Color) -> iced::widget::Container<'static, Message> {
    container(
        text("")
            .size(1)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .style(move |_theme: &Theme| {
        container::Style::default()
            .background(color)
            .border(Border {
                radius: 999.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            })
    })
}

fn primary_button<'a>(label: &'a str, message: Message) -> iced::widget::Button<'a, Message> {
    button(text(label).size(17))
        .padding([16, 18])
        .style(|_theme: &Theme, _status| button::Style {
            background: Some(Color::from_rgb8(104, 139, 208).into()),
            text_color: Color::WHITE,
            border: Border {
                radius: 18.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: iced::Shadow::default(),
        })
        .on_press(message)
}

fn secondary_button<'a>(label: &'a str, message: Message) -> iced::widget::Button<'a, Message> {
    button(text(label).size(17))
        .padding([16, 18])
        .style(|_theme: &Theme, _status| button::Style {
            background: Some(Color::from_rgb8(244, 236, 228).into()),
            text_color: Color::from_rgb8(94, 78, 66),
            border: Border {
                radius: 18.0.into(),
                width: 1.0,
                color: Color::from_rgba8(203, 185, 168, 0.28),
            },
            shadow: iced::Shadow::default(),
        })
        .on_press(message)
}

fn plain_action_button<'a>(label: &'a str, message: Message) -> iced::widget::Button<'a, Message> {
    button(text(label).size(15).color(Color::from_rgb8(126, 111, 100)))
        .padding([4, 2])
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            text_color: Color::from_rgb8(126, 111, 100),
            border: Border {
                radius: 0.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: iced::Shadow::default(),
        })
        .on_press(message)
}

fn build_window_icon() -> Result<window::Icon> {
    let width = 32;
    let height = 32;
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - 16.0;
            let dy = y as f32 - 12.0;
            let distance = (dx * dx + dy * dy).sqrt();
            let is_drop = distance < 7.5 || (y > 14 && (x as i32 - 16).abs() <= (28 - y as i32));

            let (r, g, b, a) = if is_drop {
                (59, 130, 246, 255)
            } else {
                (0, 0, 0, 0)
            };

            rgba.extend_from_slice(&[r, g, b, a]);
        }
    }

    window::icon::from_rgba(rgba, width, height).context("failed to build window icon")
}

fn play_drop_sound_async() {
    std::thread::spawn(|| {
        let _ = play_drop_sound();
    });
}

fn play_drop_sound() -> Result<()> {
    let (stream, handle) = OutputStream::try_default()
        .context("failed to open default audio output for popup sound")?;
    let sink = Sink::try_new(&handle).context("failed to create popup sound sink")?;
    let bytes = include_bytes!("../../../assets/sounds/water-drop.mp3");
    let source = Decoder::new(Cursor::new(bytes.as_slice()))
        .context("failed to decode embedded water-drop sound")?;
    let duration = source
        .total_duration()
        .unwrap_or_else(|| Duration::from_millis(500));
    sink.append(source);
    std::thread::sleep(duration + Duration::from_millis(50));
    drop(sink);
    drop(stream);
    Ok(())
}
