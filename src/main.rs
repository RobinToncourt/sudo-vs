#![allow(dead_code)]

mod game;

use iced::widget::button::Style;
use iced::widget::{Column, Row, button, column as col, container, radio, row, text};
use iced::{Alignment, Color, Element, Length, Task, Theme};

use log::debug;

use crate::game::{Game, GameOutcome, Number, NumberPlacingOutcome, Player};

const CELL_SIZE: f32 = 60.0;

struct App {
    game: Game,
    selected_number: Option<Number>,
}
impl Default for App {
    fn default() -> Self {
        Self {
            game: Game::new(),
            selected_number: Some(Number::One),
        }
    }
}

#[derive(Clone)]
enum Message {
    RadioNumberSelected(Number),
    ButtonClicked(usize, usize),
}

fn main() -> iced::Result {
    println!("Hello, sudo-vs!");
    init_logger();

    iced::application(App::default, update, view).run()
}

#[allow(clippy::needless_pass_by_value)]
fn update(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::RadioNumberSelected(number) => app.selected_number = Some(number),
        Message::ButtonClicked(row, column) => {
            if let Some(selected_number) = app.selected_number {
                let outcome = app.game.place_number(row, column, selected_number);
                match outcome {
                    NumberPlacingOutcome::Ok => debug!("Valid position"),
                    NumberPlacingOutcome::CellNotEmpty => {
                        debug!("Cell is not empty(should not happen)");
                    }
                    NumberPlacingOutcome::GameOver(reason) => match reason {
                        GameOutcome::Draw => debug!("Draw"),
                        GameOutcome::NumberAlreadyInSquare => debug!("already in square"),
                        GameOutcome::NumberAlreadyInRow => debug!("already in row"),
                        GameOutcome::NumberAlreadyInColumn => debug!("already in column"),
                        GameOutcome::NoValidPositions => debug!("unsupported yet"),
                    },
                }
            } else {
                debug!("No number selected");
            }
        }
    }
    Task::none()
}

fn view(app: &App) -> Element<'_, Message> {
    container(
        col![player_turn(app), show_grid(app), number_selection(app),]
            .align_x(Alignment::Center)
            .spacing(10),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn player_turn(app: &App) -> Element<'_, Message> {
    text(format!("{}'s turn", player_to_name(app.game.playing()))).into()
}

fn player_to_name(player: &Player) -> &'static str {
    match player {
        Player::Player1 => "Bob",
        Player::Player2 => "Patrick",
    }
}

fn show_grid(app: &App) -> Element<'_, Message> {
    let grid = app.game.grid();

    let mut rows = Vec::with_capacity(grid.len());
    for (x, outer) in grid.iter().enumerate() {
        let mut row: Row<'_, Message> = Row::new();
        for (y, inner) in outer.iter().enumerate() {
            if let Some(number) = inner {
                let t = text(number.to_string())
                    .width(Length::Fixed(CELL_SIZE))
                    .height(Length::Fixed(CELL_SIZE))
                    .size(30.0)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center);
                row = row.push(t);
            } else {
                let b = button("")
                    .on_press(Message::ButtonClicked(x, y))
                    .width(Length::Fixed(CELL_SIZE))
                    .height(Length::Fixed(CELL_SIZE))
                    .style(style_button);
                row = row.push(b);
            }
        }
        rows.push(row.spacing(1).into());
    }

    Column::from_iter(rows).spacing(1).into()
}

fn style_button(theme: &Theme, status: button::Status) -> Style {
    let mut style = button::primary(theme, status);

    match status {
        button::Status::Hovered => {
            style.background = Some(Color::from_rgb(0.2, 0.6, 0.9).into());
        }
        button::Status::Pressed => {
            style.background = Some(Color::from_rgb(0.1, 0.4, 0.7).into());
        }
        _ => {
            // Default state color
            style.background = Some(Color::from_rgb(0.1, 0.5, 0.8).into());
        }
    }
    style.text_color = Color::WHITE;

    style.border.radius = 0.0.into();

    style
}

fn number_selection(app: &App) -> Element<'_, Message> {
    let one = number_to_radio(app, Number::One);
    let two = number_to_radio(app, Number::Two);
    let three = number_to_radio(app, Number::Three);
    let four = number_to_radio(app, Number::Four);
    let five = number_to_radio(app, Number::Five);
    let six = number_to_radio(app, Number::Six);
    let seven = number_to_radio(app, Number::Seven);
    let eight = number_to_radio(app, Number::Eight);
    let nine = number_to_radio(app, Number::Nine);
    row![one, two, three, four, five, six, seven, eight, nine]
        .spacing(10)
        .into()
}

fn number_to_radio(app: &App, number: Number) -> Element<'_, Message> {
    col![
        text(format!("{number}  ")),
        radio(
            "",
            number,
            app.selected_number,
            Message::RadioNumberSelected,
        ),
    ]
    .align_x(Alignment::Center)
    .spacing(5)
    .into()
}

fn init_logger() {
    use tracing_subscriber::prelude::*;

    let log_level = format!("{}=debug", env!("CARGO_CRATE_NAME"));
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));
    let (filter, _handle) = tracing_subscriber::reload::Layer::new(filter);

    let fmt_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);

    tracing_subscriber::Registry::default()
        .with(filter)
        .with(fmt_layer)
        .try_init()
        .expect("Failed to initialize logger");
}
