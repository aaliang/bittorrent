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

    pub fn convert_bitfield_to_piece_vec (bitfield: &[u8]) -> Vec<Piece> {
        let mut vec = Vec::new();
        let mut a_start = None;
        for (bitmap_byte_num, byte) in bitfield.iter().enumerate() {
            let mut bitmap_offset = 0;
            let mut remainder = byte.to_owned();
            loop {
                match remainder.leading_zeros() {
                    0 => (),
                    x => {
                        let n = if x > 8 - bitmap_offset { 8 - bitmap_offset} else {x};
                        bitmap_offset += n;
                        match a_start {
                            Some(_) => {
                                let end = Position::new((bitmap_byte_num as u32 * 8 + bitmap_offset - n as u32) as usize, 0);
                                vec.push(Piece::new(a_start.unwrap(), end));
                                a_start = None;
                            },
                            None => {}
                        };
                        remainder = (((remainder as u16) << n) & 255) as u8;
                    }
                };
                match (!remainder).leading_zeros() { //leading 1's after shifting
                    0 => (),
                    n => {
                        match a_start {
                            Some(_) => {/*do nothing*/},
                            None => {
                                a_start = Some(Position::new((bitmap_byte_num as u32 * 8 + bitmap_offset as u32) as usize, 0));
                            }
                        }
                        bitmap_offset += n;
                        remainder = (((remainder as u16 )<< n) & 255) as u8;
                    }
                };
                if bitmap_offset == 8 {
                    bitmap_offset = 0;
                    break;
                };
            }
        }
        match a_start {
            Some(_) => {
                vec.push((
                    Piece::new(a_start.unwrap(), 
                    Position::new(bitfield.len() * 8, 0))));
            },
            _ => ()
        };
        vec
    }

    ///attempts to compact the piece indexed by {index} with elements to its left and right
    #[inline]
    pub fn compact_if_possible(arr: &mut Vec<Piece>, index: usize) {
        let res = {
            let ref el = arr[index];

            let tup = match index {
                0 => (None, arr.get(index+1)),
                _ => (arr.get(index-1), arr.get(index+1))
            };
            match tup {
                (Some(ref left), Some(ref right)) if left.end == el.start && el.end == right.start => {
                    Some((index-1, index+1, Piece::new(left.start.clone(), right.end.clone())))},
                (Some(ref left), _) if left.end == el.start => {
                    Some((index-1, index, Piece::new(left.start.clone(), el.end.clone())))},
                (_, Some(ref right)) if el.end == right.start => {
                    Some((index, index+1, Piece::new(el.start.clone(), right.end.clone())))}
                _ => None
            }
        };
        match res {
            Some((start_index, end_index, compacted_piece)) => {
                for (n, i) in (start_index..end_index+1).enumerate() {
                    arr.remove(i-n);
                }
                arr.insert(start_index, compacted_piece);
            },
            _ => ()
        }
    }

    #[inline]
    ///returns the index at which the chunk was inserted into the vector
    pub fn add_to_boundary_vec(arr: &mut Vec<Piece>, new_block: Piece) -> usize {
        //let new_block = DefaultHandler::get_block_boundaries(piece_length, index, offset, bytes);
        if arr.len() == 0 || new_block.start >= arr.last().unwrap().end {
            arr.push(new_block);
            arr.len() - 1
        } else if new_block.end <= arr.first().unwrap().start {
            arr.insert(0, new_block);
            0
        } else {
            let (mut win_left, mut win_right) = (0, arr.len());
            while win_left < win_right { //should probably just use loop {}
                let arr_index = (win_left+win_right)/2;
                let something = {
                    let block = &arr[arr_index];
                    let el_left = &arr[arr_index - 1];
                    let el_right = arr.get(arr_index + 1);
                    if new_block.start >= block.end {
                        match el_right {
                            a @ None | a @ Some(_) if new_block.end <= a.unwrap().start => {
                                Some(arr_index+1)
                            },
                            _ => {
                                win_left = arr_index + 1;
                                None
                            }
                        }
                    }
                    else if new_block.end <= block.start {
                        if new_block.start >= el_left.end {
                            Some(arr_index)
                        } else {
                            win_right = arr_index - 1;
                            None
                        }
                    }
                    else { panic!("this is bad")}
                };

                match something {
                    Some(i) => {
                        arr.insert(i, new_block);
                        return i
                    },
                    _ => ()
                }
            }
            //if (win_left > win_right) {
            panic!("this is also bad");
            //}
        }
    }


}

