extern crate bittorrent;

use bittorrent::default_handler::*;
use bittorrent::chunk::*;
use bittorrent::peer::*;

#[test]
fn test_nand_slice() {
    let a = vec![0, 0];
    let b = vec![0, 1];
    let c = nand_slice_vbr_len(&a, &b);

    assert_eq!(c, vec![255, 254]);
}

/*#[test]
fn test_unclaimed_fields() {
    let mut handler = DefaultHandler;

    handler.owned = vec![0, 0, 0];
    handler.request_map = vec![1, 0];

    let c = handler.unclaimed_fields();
    assert_eq!(c, vec![254, 255, 255]);
}*/

#[test]
fn test_set_have_singleton_bitfield() {
    let mut state = State::new();

    state.set_bitfield(vec![0]);
    state.set_have(2);

    assert_eq!(state.bitfield[0], 32);
}

#[test]
fn test_set_have_longer_bitfield() {
    let mut state = State::new();

    state.set_bitfield(vec![0, 0, 0, 0]);
    state.set_have(23);

    assert_eq!(state.bitfield[0], 0);
    assert_eq!(state.bitfield[1], 0);
    assert_eq!(state.bitfield[2], 1);
    assert_eq!(state.bitfield[3], 0);
}

#[test]
fn test_set_have_out_of_bounds() {
    let mut state = State::new();

    state.set_bitfield(vec![0, 1]);
    state.set_have(31);

    assert_eq!(state.bitfield[0], 0);
    assert_eq!(state.bitfield[1], 1);
    assert_eq!(state.bitfield[2], 0);
    assert_eq!(state.bitfield[3], 1);
}

#[test]
fn test_get_block_boundaries_1() {
    let piece_length = 300;
    let index = 0;
    let offset = 50;
    let bytes = 1000;
    let block = Piece::from(piece_length, index, offset, bytes);

    assert_eq!(block, Piece::new(Position::new(0, 50), Position::new(3, 150)));
}


#[test]
fn test_get_block_boundaries_2() {
    let piece_length = 5;
    let index = 0;
    let offset = 0;
    let bytes = 5;
    let block = Piece::from(piece_length, index, offset, bytes);

    assert_eq!(block, Piece::new(Position::new(0, 0), Position::new(1, 0)));
}

#[test]
fn test_get_block_boundaries_3() {
    let piece_length = 5;
    let index = 0;
    let offset = 0;
    let bytes = 6;
    let block = Piece::from(piece_length, index, offset, bytes);

    assert_eq!(block, Piece::new(Position::new(0, 0), Position::new(1, 1)));
}

#[test]
fn test_request_block () {
    let mut vec = vec![];
    let piece_length = 5;

    let a = {  //empty case
        let index = 0;
        let offset = 0;
        let bytes = 5;

        let expect = Piece::new(Position::new(index, offset), Position::new(index+1, 0));
        let block = Piece::from(piece_length, index, offset, bytes);
        Piece::add_to_boundary_vec(&mut vec, block);
        assert_eq!(vec[0], expect.clone());

        expect
    };

    let b = {
        let index = 5;
        let offset = 0;
        let bytes = 5;

        let expect = Piece::new(Position::new(index, offset), Position::new(index+1, 0));
        let block = Piece::from(piece_length, index, offset, bytes);
        Piece::add_to_boundary_vec(&mut vec, block);
        assert_eq!(vec, vec![a.clone(), expect.clone()]);

        expect
    };

    let c = {
        let index = 3;
        let offset = 0;
        let bytes = 5;

        let expect = Piece::new(Position::new(index, offset), Position::new(index+1, 0));
        let block = Piece::from(piece_length, index, offset, bytes);
        Piece::add_to_boundary_vec(&mut vec, block);
        assert_eq!(vec, vec![a.clone(), expect.clone(), b.clone()]);

        expect
    };

    let d = {
        let index = 4;
        let offset = 0;
        let bytes = 5;

        let expect = Piece::new(Position::new(index, offset), Position::new(index+1, 0));
        let block = Piece::from(piece_length, index, offset, bytes);
        Piece::add_to_boundary_vec(&mut vec, block);
        assert_eq!(vec, vec![a.clone(), c.clone(), expect.clone(), b.clone()]);

        expect
    };

}

#[test]
fn test_request_block_with_compaction () {
    let mut vec = vec![];
    let piece_length = 5;

    let _ = {
        let index = 0;
        let offset = 0;
        let bytes = 5;

        let expect = Piece::new(Position::new(index, offset), Position::new(index, piece_length - 1));
        let block = Piece::from(piece_length, index, offset, bytes);
        Piece::add_to_boundary_vec(&mut vec, block)
    };

    let index = {
        let index = 1;
        let offset = 0;
        let bytes = 5;

        let expect = Piece::new(Position::new(index, offset), Position::new(index, piece_length - 1));
        let block = Piece::from(piece_length, index, offset, bytes);
        Piece::add_to_boundary_vec(&mut vec, block)
    };

    Piece::compact_if_possible(&mut vec, index);

    assert_eq!(vec, vec![Piece::new(Position::new(0, 0), Position::new(2, 0))]);
}

#[test]
pub fn test_convert_bitfield_to_piece_vec() {
    let p = Piece::convert_bitfield_to_piece_vec(&vec![1, 1]);
    assert_eq!(p, vec![Piece::new(Position::new(7, 0), Position::new(8, 0)),
                      Piece::new(Position::new(15, 0), Position::new(16, 0))]);

    let a = Piece::convert_bitfield_to_piece_vec(&vec![128]);
    assert_eq!(a, vec![Piece::new(Position::new(0, 0), Position::new(1, 0))]);
}

#[test]
pub fn test_trivial_complement() {
    let a = vec![
        Piece::new(Position::new(0, 0), Position::new(3, 0))
        ];

    let b = vec![
        Piece::new(Position::new(0, 0), Position::new(1, 0))
    ];

    let c = Piece::complement(&a, &b);

    assert_eq!(c, vec![Piece::new(Position::new(1, 0), Position::new(3, 0))]);
}

#[test]
pub fn test_complement_2 () {
    let a = vec![
        Piece::create((1, 0), (5, 0))
    ];
    let b = vec![
        Piece::create((3, 0), (10, 0))
    ];
    let c = Piece::complement(&a, &b);
    assert_eq!(c, vec![Piece::create((1, 0),(3, 0))]);
}
