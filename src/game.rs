use std::fmt;

pub enum Player {
    Player1,
    Player2,
}
impl Player {
    /// Swap player.
    fn next(&mut self) {
        match self {
            Player::Player1 => *self = Player::Player2,
            Player::Player2 => *self = Player::Player1,
        }
    }
}

pub struct InvalidNumberConversion;

#[derive(Clone, Copy, PartialEq, Eq)]
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
    fn from(number: Number) -> u8 {
        match number {
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
    type Error = InvalidNumberConversion;

    fn try_from(number: u8) -> Result<Number, Self::Error> {
        match number {
            1 => Ok(Number::One),
            2 => Ok(Number::Two),
            3 => Ok(Number::Three),
            4 => Ok(Number::Four),
            5 => Ok(Number::Five),
            6 => Ok(Number::Six),
            7 => Ok(Number::Seven),
            8 => Ok(Number::Eight),
            9 => Ok(Number::Nine),
            _ => Err(InvalidNumberConversion),
        }
    }
}
impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", u8::from(*self))
    }
}

/// The result of placing a number.
pub enum NumberPlacingOutcome {
    /// The number placement is valid.
    Ok,
    /// The cell is already filled, don't change the playing player.
    CellNotEmpty,
    /// The game is over, contains the result.
    GameOver(GameOutcome),
}

/// The outcome of a finished game.
pub enum GameOutcome {
    /// The Sudoku grid is full and valid, no player won.
    Draw,
    /// The placed number already exists in this square.
    NumberAlreadyInSquare,
    /// The placed number already exists in this row.
    NumberAlreadyInRow,
    /// The placed number already exists in this column.
    NumberAlreadyInColumn,
    /// The player has no remaining valid position, next move will violate the Sudoku's rules.
    NoValidPositions,
}

type Line = [Option<Number>; 9];

pub struct Game {
    /// The player to do the next move.
    playing: Player,
    // [
    //     [0,0; 0,1; 0,2;...]
    //     [1,0; 1,1; 1,2;...]
    //     [2,0; 2,1; 2,2;...]
    //     [3,0; 3,1; 3,2;...]
    //     ...
    // ]
    // grid[x] returns a column.
    // grid[x][y]
    grid: [[Option<Number>; 9]; 9],
}
impl Game {
    pub fn new() -> Self {
        Self {
            playing: Player::Player1,
            grid: [[None; 9]; 9],
        }
    }
    /// Returns the player that will do the next move.
    pub fn playing(&self) -> &Player {
        &self.playing
    }
    /// Returns the grid.
    pub fn grid(&self) -> &[[Option<Number>; 9]; 9] {
        &self.grid
    }
    #[must_use]
    pub fn place_number(&mut self, x: usize, y: usize, number: Number) -> NumberPlacingOutcome {
        if self.grid[x][y].is_none() {
            if let Some(game_outcome) = self.check_place_number(x, y, number) {
                return NumberPlacingOutcome::GameOver(game_outcome);
            }

            self.grid[x][y] = Some(number);
            self.playing.next();

            if self.is_grid_full() {
                return NumberPlacingOutcome::GameOver(GameOutcome::Draw);
            }
            // TODO: check if at least one valid position remains.

            NumberPlacingOutcome::Ok
        } else {
            NumberPlacingOutcome::CellNotEmpty
        }
    }
    fn is_grid_full(&self) -> bool {
        for outer in &self.grid {
            for inner in outer {
                if inner.is_none() {
                    return false;
                }
            }
        }

        true
    }
    /// Returns the percent of the grid filled.
    fn percent_fill(&self) -> f32 {
        let mut sum: u8 = 0;

        for outer in &self.grid {
            for inner in outer {
                if inner.is_some() {
                    sum += 1;
                }
            }
        }

        f32::from(sum) / 81.0
    }
    /// Verify that the number placement does not violate Sudoku's rules.
    fn check_place_number(&self, x: usize, y: usize, number: Number) -> Option<GameOutcome> {
        if self.numbers_in_row(x).contains(&number) {
            return Some(GameOutcome::NumberAlreadyInRow);
        }
        if self.numbers_in_col(y).contains(&number) {
            return Some(GameOutcome::NumberAlreadyInColumn);
        }
        let (square_x, square_y) = number_position_to_square_position(x, y);
        if self.numbers_in_square(square_x, square_y).contains(&number) {
            return Some(GameOutcome::NumberAlreadyInSquare);
        }

        None
    }
    /// Returns the `Number`s in this row.
    fn numbers_in_row(&self, x: usize) -> Vec<Number> {
        let mut result = Vec::with_capacity(8);

        for number in self.grid[x].iter().flatten() {
            result.push(*number);
        }

        result
    }
    /// Returns the `Number`s in this col.
    fn numbers_in_col(&self, y: usize) -> Vec<Number> {
        let mut result = Vec::with_capacity(8);

        for column in self.grid {
            if let Some(number) = column[y] {
                result.push(number);
            }
        }

        result
    }
    /// Returns the `Number`s in this square.
    fn numbers_in_square(&self, square_x: usize, square_y: usize) -> Vec<Number> {
        let mut result = Vec::with_capacity(8);

        for i in square_x * 3..square_x * 3 + 3 {
            for j in square_y * 3..square_y * 3 + 3 {
                if let Some(number) = self.grid[i][j] {
                    result.push(number);
                }
            }
        }

        result
    }
}

/// Convert a position in ths Sudoku grid to the square position that contains it.
fn number_position_to_square_position(x: usize, y: usize) -> (usize, usize) {
    (x / 3, y / 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number_position_to_square_position() {
        assert_eq!(number_position_to_square_position(0, 0), (0, 0));
        assert_eq!(number_position_to_square_position(2, 2), (0, 0));
        assert_eq!(number_position_to_square_position(3, 3), (1, 1));
        assert_eq!(number_position_to_square_position(5, 5), (1, 1));
        assert_eq!(number_position_to_square_position(6, 6), (2, 2));
        assert_eq!(number_position_to_square_position(8, 8), (2, 2));
    }

    #[test]
    fn test_numbers_in_square() {
        let mut game = Game::new();
        game.grid[4][5] = Some(Number::Seven); // square (1, 1)
        assert!(game.numbers_in_square(1, 1).contains(&Number::Seven));
        assert!(!game.numbers_in_square(0, 0).contains(&Number::Seven));
    }
}
