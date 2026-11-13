use std::time::Duration;

use iced::{
    Alignment,
    Color,
    Element,
    Length,
    Theme, //
    widget::{
        Column,
        Row,
        button,
        button::Style,
        column as col,
        container,
        radio,
        row,
        scrollable,
        space,
        text,
        text_input, //
    },
};
use iced_aw::DropDown;

use crate::{
    App,
    CELL_SIZE,
    CELL_SPACING,
    GameOverInfo,
    GameState,
    LAST_POSITION_COLOR,
    LobbyInfo,
    Localization,
    Message,
    SQUARE_SPACING,
    State,
    game::{
        File,
        Game,
        GameResult,
        Grid,
        LossReason,
        Number,
        Player,
        Position,
        Rank,
        RuleViolation, //
    },
    is_player_turn, //
};

pub fn view(app: &App) -> Element<'_, Message> {
    let top_bar = top_bar(app);

    let main_element = match &app.state {
        State::SelectingLobby { info, .. } => {
            let nickname = &app.nickname;
            let lobby_name = &app.lobby_name;
            let signaling_server_url = &app.signaling_server_url;
            view_selecting_lobby(
                app.localization,
                *info,
                nickname,
                lobby_name,
                signaling_server_url,
            )
        }
        State::WaitingForSocket | State::WaitingForPeer { .. } | State::Handshaking { .. } => {
            view_connecting(app.localization)
        }
        State::Game(g) => view_game(app.localization, &app.nickname, g),
        State::GameOver {
            info,
            is_playing_first,
            game,
            last_placement,
            ..
        } => view_game_over(
            app.localization,
            *info,
            *is_playing_first,
            game,
            *last_placement,
        ),
    };

    col![top_bar, main_element,].into()
}

fn top_bar(app: &App) -> Element<'_, Message> {
    let theme_menu = theme_menu(app);
    let language_menu = language_menu(app);

    row![theme_menu, language_menu].into()
}

fn theme_menu(app: &App) -> Element<'_, Message> {
    let underlay = Row::new()
        .push(button(text(format!("{}", app.theme))).on_press(Message::ThemeExpand))
        .spacing(8)
        .align_y(Alignment::Center);
    let options = Column::with_children(Theme::ALL.iter().map(|theme| {
        Row::new()
            .push(button(text(theme.to_string())).on_press(Message::SelectedTheme(theme.clone())))
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    }));
    let overlay = scrollable(container(options.padding([0, 4])).style(container::bordered_box));

    let theme_drop_down = DropDown::new(underlay, overlay, app.theme_expanded);
    theme_drop_down.into()
}

fn language_menu(app: &App) -> Element<'_, Message> {
    let underlay = Row::new()
        .push(button(text(format!("{}", app.localization))).on_press(Message::LocalizationExpand))
        .spacing(8)
        .align_y(Alignment::Center);
    let options = Column::with_children(Localization::ALL.iter().map(|localization| {
        Row::new()
            .push(
                button(text(localization.to_string()))
                    .on_press(Message::SelectedLocalization(*localization)),
            )
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    }));
    let overlay = scrollable(container(options.padding([0, 4])).style(container::bordered_box));

    let localization_drop_down = DropDown::new(underlay, overlay, app.localization_expanded);
    localization_drop_down.into()
}

fn view_selecting_lobby<'a>(
    loc: Localization,
    info: LobbyInfo,
    nickname: &'a str,
    lobby_name: &'a str,
    signaling_server_url: &'a str,
) -> Element<'a, Message> {
    container(
        col![
            // Info.
            text(info.to_str(loc)),
            // Nickname.
            row![
                space().width(Length::Fill),
                text(loc.nickname())
                    .width(Length::Fixed(90.0))
                    .align_x(Alignment::End),
                text_input(loc.nickname_placeholder(), nickname)
                    .on_input(Message::InputNickname)
                    .width(Length::Fill),
                space().width(Length::Fill),
            ]
            .align_y(Alignment::Center),
            // Lobby name.
            row![
                space().width(Length::Fill),
                text(loc.lobby())
                    .width(Length::Fixed(90.0))
                    .align_x(Alignment::End),
                text_input(loc.lobby_placeholder(), lobby_name)
                    .on_input(Message::InputLobbyName)
                    .width(Length::Fill),
                space().width(Length::Fill),
            ]
            .align_y(Alignment::Center),
            // TODO: add text to signal wrong lobby name.
            // Signaling server url.
            // TODO: toggle hide/reveal the text_input.
            row![
                space().width(Length::Fill),
                text(loc.signaling_server_url())
                    .width(Length::Fixed(90.0))
                    .align_x(Alignment::End),
                text_input(loc.signaling_server_url_placeholder(), signaling_server_url)
                    .on_input(Message::InputSignalingServer)
                    .width(Length::Fill),
                space().width(Length::Fill),
            ]
            .align_y(Alignment::Center),
            // Join button.
            button(loc.join())
                .on_press_maybe(can_join_lobby(nickname, lobby_name).then_some(Message::JoinLobby))
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn can_join_lobby(nickname: &str, lobby_name: &str) -> bool {
    let is_all_alphanumeric = lobby_name.chars().all(char::is_alphanumeric);
    !nickname.is_empty() && !lobby_name.is_empty() && is_all_alphanumeric
}

fn view_connecting(localization: Localization) -> Element<'static, Message> {
    container(
        col![
            text(localization.waiting_connection()).size(40),
            button(localization.cancel()).on_press(Message::CancelConnection),
        ]
        .align_x(Alignment::Center),
    )
    .center(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn view_game<'a>(loc: Localization, nickname: &'a str, g: &'a GameState) -> Element<'a, Message> {
    let my_turn = is_player_turn(g.is_playing_first, g.game.active_player());
    container(
        col![
            player_turn(nickname, g),
            text(format!(
                "{nickname}: {} | {}: {}",
                fmt_clock(g.clock.mine(my_turn, g.now)),
                g.opponent_nickname,
                fmt_clock(g.clock.theirs(my_turn, g.now)),
            )),
            show_grid(
                &g.game,
                GridInteraction::Interactive,
                &option_to_vec(g.last_placement)
            ),
            number_selection(g),
            button(loc.forfeit()).on_press(Message::Forfeit),
        ]
        .align_x(Alignment::Center)
        .spacing(10),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn view_game_over(
    loc: Localization,
    info: GameOverInfo,
    is_player_first: bool,
    g: &Game,
    last_placement: Option<Position>,
) -> Element<'_, Message> {
    let mut end_text = String::new();
    let mut highlight_positions = Vec::new();
    match info {
        GameOverInfo::Finished => match g.result() {
            Some(GameResult::Draw) => {
                end_text.push_str(loc.draw());
            }
            Some(GameResult::Loss {
                loser,
                reason: LossReason::RuleViolation(RuleViolation { conflicting, .. }),
            }) => {
                if is_player_turn(is_player_first, loser) {
                    end_text.push_str(loc.player_lost());
                } else {
                    end_text.push_str(loc.player_win());
                }
                highlight_positions.push(conflicting);
                if let Some(position) = last_placement {
                    highlight_positions.push(position);
                }
            }
            Some(GameResult::Loss {
                loser,
                reason: LossReason::NoValidPositions,
            }) => {
                if is_player_turn(is_player_first, loser) {
                    end_text.push_str(loc.player_lost());
                } else {
                    end_text.push_str(loc.player_win());
                }
            }
            None => {
                end_text.push_str(loc.game_finished());
            }
        },
        _ => {
            end_text.push_str(info.to_str(loc));
        }
    }
    container(
        col![
            text(end_text),
            show_grid(g, GridInteraction::None, &highlight_positions),
            // TODO: button to save history.
            button(loc.return_to_lobby()).on_press(Message::ReturnToLobby),
        ]
        .align_x(Alignment::Center)
        .spacing(10),
    )
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .into()
}

fn option_to_vec<T>(opt: Option<T>) -> Vec<T> {
    let mut result = Vec::new();
    if let Some(opt) = opt {
        result.push(opt);
    }
    result
}

fn player_turn<'a>(nickname: &'a str, g: &GameState) -> Element<'a, Message> {
    text(format!(
        "{}'s turn",
        player_to_name(
            nickname,
            &g.opponent_nickname,
            g.is_playing_first,
            g.game.active_player()
        )
    ))
    .into()
}

fn player_to_name<'a>(
    nickname: &'a str,
    opponent_nickname: &'a str,
    is_playing_first: bool,
    player: Player,
) -> &'a str {
    if is_player_turn(is_playing_first, player) {
        nickname
    } else {
        opponent_nickname
    }
}

#[derive(Debug, Clone, Copy)]
enum GridInteraction {
    Interactive,
    None,
}

fn show_grid<'g>(
    g: &'g Game,
    grid_interaction: GridInteraction,
    highlight_position: &[Position],
) -> Element<'g, Message> {
    let grid = g.grid();

    Column::with_children([File::A, File::D, File::G].map(|file| {
        Row::with_children(
            [Rank::One, Rank::Four, Rank::Seven]
                .map(|rank| show_square(grid, rank, file, grid_interaction, highlight_position)),
        )
        .spacing(SQUARE_SPACING)
        .into()
    }))
    .spacing(SQUARE_SPACING)
    .into()
}

fn show_square<'g>(
    grid: &'g Grid,
    rank: Rank,
    file: File,
    grid_interaction: GridInteraction,
    highlight_position: &[Position],
) -> Element<'g, Message> {
    let ranks = rank.band();
    let files = file.stack();
    Column::with_children(files.iter().map(|&d_file| {
        Row::with_children(ranks.iter().map(|&d_rank| {
            let position = Position::new(d_rank, d_file);

            if let Some(number) = grid.get(position) {
                let t = text(number.to_string())
                    .width(Length::Fixed(CELL_SIZE))
                    .height(Length::Fixed(CELL_SIZE))
                    .size(30.0)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center);

                if highlight_position.contains(&position) {
                    container(t)
                        .style(|_: &Theme| container::Style {
                            background: Some(LAST_POSITION_COLOR.into()),
                            ..container::Style::default()
                        })
                        .into()
                } else {
                    t.into()
                }
            } else {
                let mut button = button("")
                    .width(Length::Fixed(CELL_SIZE))
                    .height(Length::Fixed(CELL_SIZE))
                    .style(style_button);

                if let GridInteraction::Interactive = grid_interaction {
                    button = button.on_press(Message::SudokuButtonClicked(position));
                }

                button.into()
            }
        }))
        .spacing(CELL_SPACING)
        .into()
    }))
    .spacing(CELL_SPACING)
    .into()
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

fn number_selection(g: &GameState) -> Element<'_, Message> {
    // TODO: use Number::All in a for loop.
    let one = number_to_radio(g.selected_number, Number::One);
    let two = number_to_radio(g.selected_number, Number::Two);
    let three = number_to_radio(g.selected_number, Number::Three);
    let four = number_to_radio(g.selected_number, Number::Four);
    let five = number_to_radio(g.selected_number, Number::Five);
    let six = number_to_radio(g.selected_number, Number::Six);
    let seven = number_to_radio(g.selected_number, Number::Seven);
    let eight = number_to_radio(g.selected_number, Number::Eight);
    let nine = number_to_radio(g.selected_number, Number::Nine);
    row![one, two, three, four, five, six, seven, eight, nine]
        .spacing(10)
        .into()
}

fn number_to_radio(selected_number: Option<Number>, number: Number) -> Element<'static, Message> {
    col![
        text(format!("{number}  ")),
        radio("", number, selected_number, Message::RadioNumberSelected,),
    ]
    .align_x(Alignment::Center)
    .spacing(5)
    .into()
}

fn fmt_clock(d: Duration) -> String {
    let s = d.as_secs() + u64::from(d.subsec_nanos() > 0);
    format!("{}:{:02}", s / 60, s % 60)
}
