use std::{
    error::Error,
    fmt::{
        self,
        Display, //
    },
    str::FromStr,
};

use serde::{
    Deserialize,
    Serialize, //
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    #[default]
    Player1,
    Player2,
}

#[derive(Debug, Clone, Copy)]
pub struct InvalidConversion;
impl Display for InvalidConversion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid input format")
    }
}
impl Error for InvalidConversion {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rank {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}
impl From<Rank> for u8 {
    fn from(value: Rank) -> Self {
        match value {
            Rank::One => 1,
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
        }
    }
}
impl TryFrom<u8> for Rank {
    type Error = InvalidConversion;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Rank::One),
            2 => Ok(Rank::Two),
            3 => Ok(Rank::Three),
            4 => Ok(Rank::Four),
            5 => Ok(Rank::Five),
            6 => Ok(Rank::Six),
            7 => Ok(Rank::Seven),
            8 => Ok(Rank::Eight),
            9 => Ok(Rank::Nine),
            _ => Err(InvalidConversion),
        }
    }
}
impl Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", u8::from(*self))
    }
}
impl Rank {
    pub const ALL: &'static [Self] = &[
        Self::One,
        Self::Two,
        Self::Three,
        Self::Four,
        Self::Five,
        Self::Six,
        Self::Seven,
        Self::Eight,
        Self::Nine,
    ];
    /// One => 0, Two => 1, ..., Nine => 8.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Rank::One => 0,
            Rank::Two => 1,
            Rank::Three => 2,
            Rank::Four => 3,
            Rank::Five => 4,
            Rank::Six => 5,
            Rank::Seven => 6,
            Rank::Eight => 7,
            Rank::Nine => 8,
        }
    }
    /// The three rows that share a square with `self`.
    #[must_use]
    pub fn band(self) -> &'static [Self] {
        let s = self as usize / 3 * 3;
        &Self::ALL[s..s + 3]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum File {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
}
impl From<File> for u8 {
    fn from(value: File) -> Self {
        match value {
            File::A => 1,
            File::B => 2,
            File::C => 3,
            File::D => 4,
            File::E => 5,
            File::F => 6,
            File::G => 7,
            File::H => 8,
            File::I => 9,
        }
    }
}
impl TryFrom<u8> for File {
    type Error = InvalidConversion;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(File::A),
            2 => Ok(File::B),
            3 => Ok(File::C),
            4 => Ok(File::D),
            5 => Ok(File::E),
            6 => Ok(File::F),
            7 => Ok(File::G),
            8 => Ok(File::H),
            9 => Ok(File::I),
            _ => Err(InvalidConversion),
        }
    }
}
impl TryFrom<&str> for File {
    type Error = InvalidConversion;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "A" => Ok(File::A),
            "B" => Ok(File::B),
            "C" => Ok(File::C),
            "D" => Ok(File::D),
            "E" => Ok(File::E),
            "F" => Ok(File::F),
            "G" => Ok(File::G),
            "H" => Ok(File::H),
            "I" => Ok(File::I),
            _ => Err(InvalidConversion),
        }
    }
}
impl Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                File::A => "A",
                File::B => "B",
                File::C => "C",
                File::D => "D",
                File::E => "E",
                File::F => "F",
                File::G => "G",
                File::H => "H",
                File::I => "I",
            }
        )
    }
}
impl File {
    pub const ALL: &'static [Self] = &[
        Self::A,
        Self::B,
        Self::C,
        Self::D,
        Self::E,
        Self::F,
        Self::G,
        Self::H,
        Self::I,
    ];
    /// A => 0, B => 1, ..., I => 8.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            File::A => 0,
            File::B => 1,
            File::C => 2,
            File::D => 3,
            File::E => 4,
            File::F => 5,
            File::G => 6,
            File::H => 7,
            File::I => 8,
        }
    }
    /// The three columns that share a square with `self`.
    #[must_use]
    pub fn stack(self) -> &'static [Self] {
        let s = self as usize / 3 * 3;
        &Self::ALL[s..s + 3]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Number {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}
impl From<Number> for u8 {
    fn from(value: Number) -> Self {
        match value {
            Number::One => 1,
            Number::Two => 2,
            Number::Three => 3,
            Number::Four => 4,
            Number::Five => 5,
            Number::Six => 6,
            Number::Seven => 7,
            Number::Eight => 8,
            Number::Nine => 9,
        }
    }
}
impl TryFrom<u8> for Number {
    type Error = InvalidConversion;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            4 => Ok(Self::Four),
            5 => Ok(Self::Five),
            6 => Ok(Self::Six),
            7 => Ok(Self::Seven),
            8 => Ok(Self::Eight),
            9 => Ok(Self::Nine),
            _ => Err(InvalidConversion),
        }
    }
}
impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", u8::from(*self))
    }
}
impl Number {
    pub const ALL: &'static [Self] = &[
        Self::One,
        Self::Two,
        Self::Three,
        Self::Four,
        Self::Five,
        Self::Six,
        Self::Seven,
        Self::Eight,
        Self::Nine,
    ];
}

/// The result of placing a number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Continue,
    Finished(GameResult),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameResult {
    Draw,
    Loss { loser: Player, reason: LossReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossReason {
    RuleViolation(RuleViolation),
    /// The player has no remaining valid position, next move will violate the Sudoku's rules.
    NoValidPositions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleViolation {
    pub rule: Rule,
    pub conflicting: Position,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    NumberAlreadyInRank,
    NumberAlreadyInFile,
    NumberAlreadyInSquare,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    CellNotEmpty,
    GameAlreadyOver,
}
impl Display for PlaceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                PlaceError::CellNotEmpty => "Cell is not empty",
                PlaceError::GameAlreadyOver => "Game is already over",
            }
        )
    }
}
impl Error for PlaceError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub rank: Rank,
    pub file: File,
}
impl Position {
    #[must_use]
    pub const fn new(rank: Rank, file: File) -> Self {
        Self { rank, file }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Grid {
    // [
    // row_1: [],
    // row_2: [],
    // row_3: [],
    //     ..
    // row_9: [],
    // ]
    cells: [[Option<Number>; 9]; 9],
}
impl std::ops::Index<Rank> for Grid {
    type Output = [Option<Number>; 9];

    fn index(&self, rank: Rank) -> &Self::Output {
        &self.cells[rank as usize]
    }
}
impl std::ops::Index<File> for [Option<Number>; 9] {
    type Output = Option<Number>;

    fn index(&self, file: File) -> &Self::Output {
        &self[file as usize]
    }
}
impl Grid {
    #[must_use]
    pub fn get(&self, position: Position) -> Option<Number> {
        let Position { rank, file } = position;
        self.cells[rank as usize][file as usize]
    }
    #[must_use]
    pub fn is_grid_full(&self) -> bool {
        for row in &self.cells {
            for cell in row {
                if cell.is_none() {
                    return false;
                }
            }
        }

        true
    }
    pub fn rank(&self, rank: Rank) -> impl Iterator<Item = (Position, Number)> + '_ {
        File::ALL
            .iter()
            .filter_map(move |&file| self.filled(rank, file))
    }
    pub fn file(&self, file: File) -> impl Iterator<Item = (Position, Number)> + '_ {
        Rank::ALL
            .iter()
            .filter_map(move |&rank| self.filled(rank, file))
    }
    pub fn square(&self, at: Position) -> impl Iterator<Item = (Position, Number)> + '_ {
        at.rank.band().iter().flat_map(move |&rank| {
            at.file
                .stack()
                .iter()
                .filter_map(move |&file| self.filled(rank, file))
        })
    }
    fn filled(&self, rank: Rank, file: File) -> Option<(Position, Number)> {
        self[rank][file].map(|number| (Position { rank, file }, number))
    }
}

/// A placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    position: Position,
    number: Number,
}
impl Placement {
    #[must_use]
    pub const fn new(position: Position, number: Number) -> Self {
        Self { position, number }
    }
    #[must_use]
    pub const fn position(self) -> Position {
        self.position
    }
}
impl Display for Placement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { position, number } = self;
        write!(f, "{}-{}-{number}", position.rank, position.file)
    }
}
impl FromStr for Placement {
    type Err = InvalidConversion;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.split('-').collect::<Vec<_>>();
        if s.len() != 3 {
            return Err(InvalidConversion);
        }

        let rank = s[0].parse::<u8>().map_err(|_| InvalidConversion)?;
        let file = s[1];
        let number = s[2].parse::<u8>().map_err(|_| InvalidConversion)?;

        let rank = Rank::try_from(rank)?;
        let file = File::try_from(file)?;
        let number = Number::try_from(number)?;

        Ok(Self {
            position: Position { rank, file },
            number,
        })
    }
}

/// The history of placement.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct History {
    history: Vec<Placement>,
}
impl Display for History {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { history } = self;
        for placement in history {
            write!(f, "{placement};")?;
        }
        Ok(())
    }
}
impl FromStr for History {
    type Err = InvalidConversion;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut history = Vec::new();

        let split = s.split(';');
        for s in split {
            if s.is_empty() {
                continue;
            }
            history.push(s.parse::<Placement>()?);
        }

        Ok(Self { history })
    }
}

#[derive(Debug, Default, Clone)]
pub struct Game {
    grid: Grid,
    /// The history of placements.
    history: History,
    result: Option<GameResult>,
}
impl Game {
    /// Returns the active player.
    #[must_use]
    pub fn active_player(&self) -> Player {
        if self.history.history.len().is_multiple_of(2) {
            Player::Player1
        } else {
            Player::Player2
        }
    }
    #[must_use]
    pub fn is_cell_empty(&self, position: Position) -> bool {
        self.grid.get(position).is_none()
    }
    /// Place a number on the grid.
    ///
    /// # Errors
    ///
    /// Returns an error if the cell is not empty or if the game is already over.
    pub fn place_number(&mut self, placement: Placement) -> Result<Outcome, PlaceError> {
        let Placement { position, number } = placement;
        let Position { rank, file } = position;

        if self.result.is_some() {
            return Err(PlaceError::GameAlreadyOver);
        }

        if !self.is_cell_empty(position) {
            return Err(PlaceError::CellNotEmpty);
        }

        // Keep the actual player of the move in case of violation.
        let player = self.active_player();
        self.history.history.push(placement);

        if let Some(rule_violation) = self.check_rules(position, number) {
            let game_result = GameResult::Loss {
                loser: player,
                reason: LossReason::RuleViolation(rule_violation),
            };
            self.grid.cells[rank as usize][file as usize] = Some(number);

            self.result = Some(game_result);
            return Ok(Outcome::Finished(game_result));
        }

        self.grid.cells[rank as usize][file as usize] = Some(number);

        if self.grid.is_grid_full() {
            self.result = Some(GameResult::Draw);
            return Ok(Outcome::Finished(GameResult::Draw));
        }

        // TODO: check if a valid move remain.

        Ok(Outcome::Continue)
    }
    fn check_rules(&self, position: Position, number: Number) -> Option<RuleViolation> {
        let Position { rank, file } = position;
        for (pos, num) in self.grid.rank(rank) {
            if num == number {
                return Some(RuleViolation {
                    conflicting: pos,
                    rule: Rule::NumberAlreadyInRank,
                });
            }
        }
        for (pos, num) in self.grid.file(file) {
            if num == number {
                return Some(RuleViolation {
                    conflicting: pos,
                    rule: Rule::NumberAlreadyInFile,
                });
            }
        }
        for (pos, num) in self.grid.square(position) {
            if num == number {
                return Some(RuleViolation {
                    conflicting: pos,
                    rule: Rule::NumberAlreadyInSquare,
                });
            }
        }

        None
    }
    #[must_use]
    pub fn grid(&self) -> &Grid {
        &self.grid
    }
    #[must_use]
    pub fn result(&self) -> Option<GameResult> {
        self.result
    }
    #[must_use]
    pub fn _history(&self) -> &History {
        &self.history
    }
    #[must_use]
    pub fn ply(&self) -> usize {
        self.history.history.len()
    }
}

#[cfg(test)]
mod game_tests {
    use super::*;

    const EMPTY: [Option<Number>; 9] = [None; 9];

    #[test]
    fn text_rank_file_index() {
        for (i, f) in Rank::ALL.iter().enumerate() {
            assert_eq!(f.index(), i);
        }
        for (i, f) in File::ALL.iter().enumerate() {
            assert_eq!(f.index(), i);
        }
    }

    #[test]
    fn test_rank_band() {
        assert_eq!(Rank::Three.band(), [Rank::One, Rank::Two, Rank::Three]);
        assert_eq!(Rank::Four.band(), [Rank::Four, Rank::Five, Rank::Six]);
        assert_eq!(Rank::Eight.band(), [Rank::Seven, Rank::Eight, Rank::Nine]);
    }

    #[test]
    fn test_file_stack() {
        assert_eq!(File::C.stack(), [File::A, File::B, File::C]);
        assert_eq!(File::D.stack(), [File::D, File::E, File::F]);
        assert_eq!(File::H.stack(), [File::G, File::H, File::I]);
    }

    #[test]
    fn test_grid_get() {
        let row = [
            Some(Number::One),
            Some(Number::Two),
            Some(Number::Three),
            Some(Number::Four),
            Some(Number::Five),
            Some(Number::Six),
            Some(Number::Seven),
            Some(Number::Eight),
            Some(Number::Nine),
        ];
        let cells: [[Option<Number>; 9]; 9] = [row, row, row, row, row, row, row, row, row];
        let grid = Grid { cells };

        assert_eq!(
            grid.get(Position::new(Rank::One, File::A)),
            Some(Number::One),
        );
        assert_eq!(
            grid.get(Position::new(Rank::One, File::E)),
            Some(Number::Five),
        );
        assert_eq!(
            grid.get(Position::new(Rank::One, File::I)),
            Some(Number::Nine),
        );
        assert_eq!(
            grid.get(Position::new(Rank::Five, File::A)),
            Some(Number::One),
        );
        assert_eq!(
            grid.get(Position::new(Rank::Nine, File::A)),
            Some(Number::One),
        );
        assert_eq!(
            grid.get(Position::new(Rank::Nine, File::I)),
            Some(Number::Nine),
        );
    }

    #[test]
    fn test_grid_row() {
        let row = [
            Some(Number::One),
            None,
            None,
            None,
            None,
            Some(Number::Six),
            Some(Number::Seven),
            None,
            None,
        ];
        let cells: [[Option<Number>; 9]; 9] =
            [row, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY];
        let grid = Grid { cells };

        let selected_row = Rank::One;
        let expected = [
            (Position::new(Rank::One, File::A), Number::One),
            (Position::new(Rank::One, File::F), Number::Six),
            (Position::new(Rank::One, File::G), Number::Seven),
        ];
        assert!(grid.rank(selected_row).eq(expected.into_iter()));
    }

    #[test]
    fn test_grid_column() {
        let cells: [[Option<Number>; 9]; 9] = [
            EMPTY,
            EMPTY,
            EMPTY,
            EMPTY,
            EMPTY,
            [
                Some(Number::Four),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            [
                Some(Number::Two),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            EMPTY,
            EMPTY,
        ];
        let grid = Grid { cells };

        let selected_column = File::A;
        let expected = [
            (Position::new(Rank::Six, File::A), Number::Four),
            (Position::new(Rank::Seven, File::A), Number::Two),
        ];
        assert!(grid.file(selected_column).eq(expected.into_iter()));
    }

    #[test]
    fn test_grid_square() {
        let cells: [[Option<Number>; 9]; 9] = [
            [
                Some(Number::One),
                Some(Number::Two),
                Some(Number::Three),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            [
                Some(Number::Four),
                Some(Number::Five),
                Some(Number::Six),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            [
                Some(Number::Seven),
                Some(Number::Eight),
                Some(Number::Nine),
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            EMPTY,
            EMPTY,
            EMPTY,
            EMPTY,
            EMPTY,
            EMPTY,
        ];
        let grid = Grid { cells };

        let at = Position::new(Rank::One, File::A);
        let expected = [
            (Position::new(Rank::One, File::A), Number::One),
            (Position::new(Rank::One, File::B), Number::Two),
            (Position::new(Rank::One, File::C), Number::Three),
            (Position::new(Rank::Two, File::A), Number::Four),
            (Position::new(Rank::Two, File::B), Number::Five),
            (Position::new(Rank::Two, File::C), Number::Six),
            (Position::new(Rank::Three, File::A), Number::Seven),
            (Position::new(Rank::Three, File::B), Number::Eight),
            (Position::new(Rank::Three, File::C), Number::Nine),
        ];
        assert!(grid.square(at).eq(expected.into_iter()));
    }

    #[test]
    fn test_placement_from_str() {
        let s = "1-B-3";
        let expected = Placement {
            position: Position {
                rank: Rank::One,
                file: File::B,
            },
            number: Number::Three,
        };
        assert_eq!(Placement::from_str(s).unwrap(), expected);
    }

    #[test]
    fn test_history_from_str() {
        let s = "1-A-5;2-B-3;";
        let history = vec![
            Placement {
                position: Position {
                    rank: Rank::One,
                    file: File::A,
                },
                number: Number::Five,
            },
            Placement {
                position: Position {
                    rank: Rank::Two,
                    file: File::B,
                },
                number: Number::Three,
            },
        ];
        let expected = History { history };
        assert_eq!(History::from_str(s).unwrap(), expected);
    }

    #[test]
    fn test_place_number_error_cell_not_empty() {
        let mut game = Game::default();
        let placement = Placement {
            position: Position::new(Rank::One, File::A),
            number: Number::One,
        };
        assert_eq!(game.place_number(placement), Ok(Outcome::Continue));
        assert_eq!(game.place_number(placement), Err(PlaceError::CellNotEmpty));
    }

    #[test]
    fn test_place_number_error_game_already_over() {
        let mut game = Game::default();
        let placement1 = Placement {
            position: Position::new(Rank::One, File::A),
            number: Number::One,
        };
        let placement2 = Placement {
            position: Position::new(Rank::Two, File::A),
            number: Number::One,
        };
        let placement3 = Placement {
            position: Position::new(Rank::Three, File::A),
            number: Number::One,
        };
        assert_eq!(game.place_number(placement1), Ok(Outcome::Continue));
        let _ = game.place_number(placement2);
        assert_eq!(
            game.place_number(placement3),
            Err(PlaceError::GameAlreadyOver)
        );
    }

    #[test]
    fn test_place_number_rule_violation_already_in_rank() {
        let mut game = Game::default();
        let placement1 = Placement {
            position: Position::new(Rank::One, File::A),
            number: Number::One,
        };
        let placement2 = Placement {
            position: Position::new(Rank::One, File::B),
            number: Number::One,
        };
        assert_eq!(game.place_number(placement1), Ok(Outcome::Continue));
        assert_eq!(
            game.place_number(placement2),
            Ok(Outcome::Finished(GameResult::Loss {
                loser: Player::Player2,
                reason: LossReason::RuleViolation(RuleViolation {
                    rule: Rule::NumberAlreadyInRank,
                    conflicting: Position {
                        rank: Rank::One,
                        file: File::A,
                    },
                })
            }))
        );
    }

    #[test]
    fn test_place_number_rule_violation_already_in_file() {
        let mut game = Game::default();
        let placement1 = Placement {
            position: Position::new(Rank::One, File::A),
            number: Number::One,
        };
        let placement2 = Placement {
            position: Position::new(Rank::Two, File::A),
            number: Number::One,
        };
        assert_eq!(game.place_number(placement1), Ok(Outcome::Continue));
        assert_eq!(
            game.place_number(placement2),
            Ok(Outcome::Finished(GameResult::Loss {
                loser: Player::Player2,
                reason: LossReason::RuleViolation(RuleViolation {
                    rule: Rule::NumberAlreadyInFile,
                    conflicting: Position {
                        rank: Rank::One,
                        file: File::A,
                    },
                })
            }))
        );
    }

    #[test]
    fn test_place_number_rule_violation_already_in_square() {
        let mut game = Game::default();
        let placement1 = Placement {
            position: Position::new(Rank::One, File::A),
            number: Number::One,
        };
        let placement2 = Placement {
            position: Position::new(Rank::Two, File::B),
            number: Number::One,
        };
        assert_eq!(game.place_number(placement1), Ok(Outcome::Continue));
        assert_eq!(
            game.place_number(placement2),
            Ok(Outcome::Finished(GameResult::Loss {
                loser: Player::Player2,
                reason: LossReason::RuleViolation(RuleViolation {
                    rule: Rule::NumberAlreadyInSquare,
                    conflicting: Position {
                        rank: Rank::One,
                        file: File::A,
                    },
                })
            }))
        );
    }

    #[test]
    #[allow(clippy::cast_possible_truncation)]
    fn test_place_number_draw() {
        let rows = [
            "534678912",
            "672195348",
            "198342567",
            "859761423",
            "426853791",
            "713924856",
            "961537284",
            "287419635",
            "345286179",
        ];
        let mut cells = [[None; 9]; 9];
        for (r, row) in rows.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                cells[r][c] = Number::try_from(ch.to_digit(10).unwrap() as u8).ok();
            }
        }
        cells[8][8] = None;
        let mut game = Game {
            grid: Grid { cells },
            ..Game::default()
        };

        let out = game.place_number(Placement::new(
            Position::new(Rank::Nine, File::I),
            Number::Nine,
        ));
        assert_eq!(out.ok(), Some(Outcome::Finished(GameResult::Draw)));
        assert_eq!(game.result(), Some(GameResult::Draw));
    }
}
