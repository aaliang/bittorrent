use std::cmp::Ordering;

#[derive(PartialEq, Debug, Clone)]
pub struct Position {
    pub index: usize,
    pub offset: usize
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

enum ItAction {
    AdvanceBoth,
    AdvanceLeft,
    AdvanceRight,
    AdvanceRightNewHeadLeft(Piece),
    ExtendWithLeftRemainder,
    NewLeftHead(Piece)
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

    /// convenience method for above. allowing you to pass in a tuple instead of a Position
    pub fn create (start: (usize, usize), end: (usize, usize)) -> Piece {
        let (a, b) = start;
        let (c, d) = end;
        Self::new(Position::new(a, b), Position::new(c, d))
    }

    pub fn num_bytes(&self, block_size: &usize) -> usize {
        (self.end.index * block_size + self.end.offset)
            - (self.start.index * block_size + self.start.offset)
    }

    //start is inclusive, end is exclusive
    pub fn from (piece_length: usize, index: usize, offset: usize, bytes: usize) -> Piece {
        println!("p {}", piece_length);
        
        let bytes_to_fill = piece_length - offset;

        if bytes < bytes_to_fill {
            Piece {
                start: Position {
                    index: index,
                    offset: offset
                },
                end: Position {
                    index: index,
                    offset: offset + bytes
                }
            }
        } else {
            let mut temp_end = Position {
                index: index+1,
                offset: 0
            };

            let rem_bytes = bytes - bytes_to_fill;
            let overflow = rem_bytes % piece_length;
            let index_o = rem_bytes / piece_length;

            temp_end.index += index_o;
            temp_end.offset = overflow;

            Piece {
                start: Position {
                    index: index, 
                    offset: offset
                },
                end: temp_end
            }
        }
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


    //TODO: it may be better to pimp the following operations on Vec<Piece> into a trait. but for
    //now they're static
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
    pub fn add_to_boundary_vec(arr: &mut Vec<Piece>, new_block: Piece) -> Result<usize, String> {
        //let new_block = DefaultHandler::get_block_boundaries(piece_length, index, offset, bytes);
        if arr.len() == 0 || new_block.start >= arr.last().unwrap().end {
            arr.push(new_block);
            Ok(arr.len() - 1)
        } else if new_block.end <= arr.first().unwrap().start {
            arr.insert(0, new_block);
            Ok(0)
        } else {
            let (mut win_left, mut win_right) = (0, arr.len());
            while win_left <= win_right { //should probably just use loop {}
                let arr_index = (win_left+win_right)/2;
                let something = {
                    let block = &arr[arr_index];
                    let el_left = if arr_index == 0 {None} else {Some(&arr[arr_index - 1])};
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
                        match el_left {
                            None => {
                                Some(0)
                            },
                            Some(a) => {
                                if new_block.start >= a.end {
                                    Some(arr_index)
                                } else {
                                    win_right = arr_index - 1;
                                None
                                }
                            }
                        }
                    }
                    else {
                        return Err(format!("veci {:?}, new_block: {:?}", arr, new_block))
                    }
                };

                match something {
                    Some(i) => {
                        arr.insert(i, new_block);
                        return Ok(i)
                    },
                    _ => ()
                }
            }
            //if (win_left > win_right) {
            println!("vec: {:?}, new_block: {:?}", arr, new_block);
            panic!("this is also bad");
            //}
        }
    }

    /// Yields the relative set complement of A in B. The vector should be compacted
    pub fn complement(a: &[Piece], b: &[Piece]) -> Vec<Piece> {
        let mut a_ptr = a.to_owned(); //TODO: need to come up with a better abstraction for lists
        let mut b_ptr = b.to_owned(); //for now go the inefficient route
        let mut vec = vec![];
        loop {
            let action = match (a_ptr.first(), b_ptr.first()) {
                (Some(ref _a), Some(ref _b)) => {
                    let (a_start, a_end) = (&_a.start, &_a.end);
                    let (b_start, b_end) = (&_b.start, &_b.end);

                    if a_start == b_start {
                        if a_end == b_end {
                            ItAction::AdvanceBoth
                        }
                        else if a_end < b_end {
                            ItAction::AdvanceLeft
                        }
                        else { //a_end > b_end
                            let new_left_head = Piece::new(b_end.to_owned(), a_end.to_owned());
                            ItAction::AdvanceRightNewHeadLeft(new_left_head)
                        }
                    }
                    else if b_start >= a_end {
                        vec.push(_a.to_owned().clone());
                        ItAction::AdvanceLeft
                    }
                    else if a_start >= b_end {
                        ItAction::AdvanceRight
                    }
                    else if a_start < b_start { //if b begins within a's boundary
                        let new_piece = Piece::new(a_start.to_owned(), b_start.to_owned());
                        vec.push(new_piece);
                        if b_end == a_end {
                            ItAction::AdvanceBoth
                        }
                        else if b_end < a_end {//b is contained within a
                            let new_a_head = Piece::new(b_end.to_owned(), a_end.to_owned());
                            ItAction::AdvanceRightNewHeadLeft(new_a_head)
                        }
                        else { //b ends outside a's boundary
                            let new_left_head = Piece::new(b_start.to_owned(), a_end.to_owned());
                            ItAction::NewLeftHead(new_left_head)
                        }
                    }
                    else if b_start < a_start {
                        if b_end == a_end {
                            ItAction::AdvanceBoth
                        }
                        else if b_end < a_end {
                            let new_a_head = Piece::new(b_end.to_owned(), a_end.to_owned());
                            ItAction::AdvanceRightNewHeadLeft(new_a_head)
                        }
                        else { //b_end > a_end
                            ItAction::AdvanceLeft
                        }
                    }
                    else {
                        panic!("this should not happen");//todo remove conditional clause if above
                    }
                },
                (Some(ref _a), None) => {
                    ItAction::ExtendWithLeftRemainder
                },
                _ => return vec
            };

            match action {
                ItAction::AdvanceBoth => {
                    a_ptr.remove(0);
                    b_ptr.remove(0);
                },
                ItAction::AdvanceLeft => {
                    a_ptr.remove(0);
                },
                ItAction::AdvanceRight => {
                    b_ptr.remove(0);
                },
                ItAction::AdvanceRightNewHeadLeft(hl) => {
                    a_ptr[0] = hl;
                    b_ptr.remove(0);
                },
                ItAction::ExtendWithLeftRemainder => {
                    vec.extend(a_ptr);
                    return vec
                },
                ItAction::NewLeftHead(hl) => {
                    a_ptr[0] = hl;
                }
            }
        }
    }

    //this calls complement twice, can be optimized to be done inline. (without using complement)
    pub fn intersection (a: &[Piece], b: &[Piece]) -> Vec<Piece> {
        match (a.last(), b.last()) {
            (Some(ref a_last), Some(ref b_last)) => {
                let end = if a_last.end >= b_last.end {a_last} else {b_last};
                let b_inverse = Piece::complement(&[Piece::new(Position::new(0, 0), end.end.clone())], b);
                println!("binverse: {:?}", b_inverse);
                Piece::complement(a, &b_inverse)

            },
            _ => vec![]
        }
    }
}

