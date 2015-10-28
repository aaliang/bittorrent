use std::cmp::Ordering;

#[derive(PartialEq, Debug, Clone)]
pub struct Position {
    index: usize,
    offset: usize
}

impl Position {
    pub fn new (index: usize, offset: usize) -> Position {
        Position {
            index: index,
            offset: offset
        }
    }
}

impl PartialOrd for Position {
    fn partial_cmp (&self, rhs: &Position) -> Option<Ordering> {
         if self.index < rhs.index {
            Some(Ordering::Less)
         } else if self.index > rhs.index {
            Some(Ordering::Greater)
         } else {
            if self.offset < rhs.offset {
                Some(Ordering::Less)
            } else if self.offset > rhs.offset {
                Some(Ordering::Greater)
            } else {
                Some(Ordering::Equal)
            }
         }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Piece {
    pub start: Position,
    pub end: Position
}

impl Piece {
    pub fn new (start: Position, end: Position) -> Piece {
        Piece {
            start: start,
            end: end
        }
    }

    //start is inclusive, end is exclusive
    pub fn from (piece_length: usize, index: usize, offset: usize, bytes: usize) -> Piece {
        let num_whole_pieces = bytes/piece_length;
        let rem_offset = (offset + bytes) % piece_length;

        let start = Position {
            index: index,
            offset: offset
        };
        let end = Position {
            index: index + num_whole_pieces,
            offset: rem_offset
        };

        Piece{start: start, end: end}
    }
}

