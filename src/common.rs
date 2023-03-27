#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Position{
    Button,
    BigBlind,
}

pub fn other_player(player: Position) -> Position{
    match player{
        Position::Button => Position::BigBlind,
        Position::BigBlind => Position::Button,
    }
}
