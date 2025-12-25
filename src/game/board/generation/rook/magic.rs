use crate::game::board::{Board, bitset::Bitset, tile::Tile};
use std::sync::OnceLock;

/// Pre-computed magic numbers for rook move generation on each square.
/// These numbers were computed via brute force search and are based on
/// well-established values used in production chess engines.
const ROOK_MAGICS: [u64; 64] = [
    0xa8002c000108020,
    0x6c00049b0002001,
    0x100200010090040,
    0x2480041000800801,
    0x280028004000800,
    0x900410008040022,
    0x280020001001080,
    0x2880002041000080,
    0xa000800080400034,
    0x4808020004000,
    0x2290802004801000,
    0x411000d00100020,
    0x402800800040080,
    0xb000401004208,
    0x2409000100040200,
    0x1002100004082,
    0x22878001e24000,
    0x1090810021004010,
    0x801030040200012,
    0x500808008001000,
    0xa08018014000880,
    0x8000808004000200,
    0x201008080010200,
    0x801020000441091,
    0x800080204005,
    0x1040200040100048,
    0x120200402082,
    0xd14880480100080,
    0x12040280080080,
    0x100040080020080,
    0x9020010080800200,
    0x813241200148449,
    0x491604001800080,
    0x100401000402001,
    0x4820010021001040,
    0x400402202000812,
    0x209009005000802,
    0x810800601800400,
    0x4301083214000150,
    0x204026458e001401,
    0x40204000808000,
    0x8001008040010020,
    0x8410820820420010,
    0x1003001000090020,
    0x804040008008080,
    0x12000810020004,
    0x1000100200040208,
    0x430000a044020001,
    0x280009023410300,
    0xe0100040002240,
    0x200100401700,
    0x2244100408008080,
    0x8000400801980,
    0x2000810040200,
    0x8010100228810400,
    0x2000009044210200,
    0x4080008040102101,
    0x40002080411d01,
    0x2005524060000901,
    0x502001008400422,
    0x489a000810200402,
    0x1004400080a13,
    0x4000011008020084,
    0x26002114058042,
];

/// Number of relevant occupancy bits for each square.
/// Used to compute the shift amount for magic multiplication.
const ROOK_SHIFTS: [u8; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12, 11, 10, 10, 10, 10, 10, 10, 11, 11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11, 11, 10, 10, 10, 10, 10, 10, 11, 11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11, 12, 11, 11, 11, 11, 11, 11, 12,
];

/// Global attack table storage. Each square has its own sub-table indexed by
/// the magic hash of the occupancy pattern.
static ATTACK_TABLE: OnceLock<Vec<Vec<Bitset>>> = OnceLock::new();

/// Relevant rook movement patterns for each square, excluding the edge squares
/// in any direction since, assuming as conventionally done that every blocker is
/// capturable, pieces on the edge don't change the moveset.
const MOVEMENT_MASKS: [Bitset; 64] = generate_movement_masks();

const fn generate_movement_masks() -> [Bitset; 64] {
    let mut masks = [Bitset::empty(); 64];

    let mut index = 0;
    while index < 64 {
        let tile = Tile::from_index(index);
        let rank = tile.rank();
        let file = tile.file();

        // mask for rank, excluding edge squares
        let mut mask = Bitset::empty();
        let mut f = 1;
        while f < Board::MAX_DIM - 1 {
            if f != file {
                mask.0 |= Bitset(1 << (rank * 8 + f)).0;
            }
            f += 1;
        }

        // mask for file, excluding edge squares
        let mut r = 1;
        while r < Board::MAX_DIM - 1 {
            if r != rank {
                mask.0 |= Bitset(1 << (r * 8 + file)).0;
            }
            r += 1;
        }

        // Combine rank and file masks
        masks[index] = mask;
        index += 1;
    }

    masks
}

/// Initializes the magic bitboard lookup tables.
/// This must be called before using `magic_rook_attacks()`.
/// Subsequent calls are no-ops (initialization happens only once).
pub fn initialize() {
    ATTACK_TABLE.get_or_init(|| {
        let mut tables = Vec::with_capacity(64);

        // for each tile...
        for tile in (0..64).map(Tile::from_index) {
            // first, get the movement pattern of a piece on this tile
            let movement_mask = MOVEMENT_MASKS[tile];

            // prepare tile's attack table holding a bitset for each possible blocker pattern
            let relevant_bits = ROOK_SHIFTS[tile];
            let table_size = 1 << relevant_bits;
            let mut tile_table = vec![Bitset::empty(); table_size];

            // for each occupancy pattern possible from this tile...
            for i in 0..(1 << movement_mask.len()) {
                // populate attack table at magic index with manually generated attacks
                let occupancy = movement_mask.occupancy(i);
                let attacks = generate_rook_attacks_slow(tile, occupancy);
                let magic_index = magic_index(tile, occupancy);
                tile_table[magic_index] = attacks;
            }

            tables.push(tile_table);
        }

        tables
    });
}

/// Returns the pre-computed rook attacks for a given tile.
pub(crate) fn magic_moves(tile: Tile, occupancy: Bitset) -> Bitset {
    let index = magic_index(tile, occupancy);
    ATTACK_TABLE
        .get()
        .expect("Magic bitboards not initialized - call Board::initialize() first")[tile][index]
}

/// Computes the magic index for a given square and occupancy pattern.
/// This is the core hash function that maps occupancy to attack table indices.
fn magic_index(tile: Tile, occupancy: Bitset) -> usize {
    let mask = MOVEMENT_MASKS[tile];
    let relevant_occupancy = occupancy & mask;
    let magic = ROOK_MAGICS[tile];
    let shift = 64 - ROOK_SHIFTS[tile];
    ((relevant_occupancy.0.wrapping_mul(magic)) >> shift) as usize
}

/// Generates rook attacks from a square given an occupancy pattern.
/// This is the "slow" version used during initialization.
fn generate_rook_attacks_slow(tile: Tile, occupancy: Bitset) -> Bitset {
    let square = tile.as_index();
    let rank = (square / 8) as i32;
    let file = (square % 8) as i32;

    let mut attacks = Bitset::empty();

    // North
    for r in (rank + 1)..8 {
        let target = Tile::from_index((r * 8 + file) as usize);
        attacks.insert(target);
        if occupancy.contains(target) {
            break;
        }
    }

    // South
    for r in (0..rank).rev() {
        let target = Tile::from_index((r * 8 + file) as usize);
        attacks.insert(target);
        if occupancy.contains(target) {
            break;
        }
    }

    // East
    for f in (file + 1)..8 {
        let target = Tile::from_index((rank * 8 + f) as usize);
        attacks.insert(target);
        if occupancy.contains(target) {
            break;
        }
    }

    // West
    for f in (0..file).rev() {
        let target = Tile::from_index((rank * 8 + f) as usize);
        attacks.insert(target);
        if occupancy.contains(target) {
            break;
        }
    }

    attacks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        initialize();
        assert!(ATTACK_TABLE.get().is_some());
    }

    #[test]
    fn test_rook_magics() {
        // asserts that all indexed moves are correctly associated
        initialize();
        for tile in (0..64).map(Tile::from_index) {
            let mask = MOVEMENT_MASKS[tile];
            let attacks = ATTACK_TABLE.get().unwrap()[tile].as_slice();

            for i in 0..(1 << mask.len()) {
                let occupancy = mask.occupancy(i);
                let expected = generate_rook_attacks_slow(tile, occupancy);

                let index = magic_index(tile, occupancy);
                let actual = attacks[index];

                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn test_occupancy_mask_center() {
        let mask = MOVEMENT_MASKS[Tile::D4];

        // Center square should have 10 relevant bits (excluding edges)
        // Files: b, c, e, f, g (5 squares on rank 4, excluding a and h)
        // Ranks: 2, 3, 5, 6, 7 (5 squares on file d, excluding 1 and 8)
        assert_eq!(mask.len(), 10);
    }

    #[test]
    fn test_occupancy_mask_corner() {
        let mask = MOVEMENT_MASKS[Tile::A1];

        // Corner square should have 12 relevant bits
        // File a: ranks 2-7 (6 squares)
        // Rank 1: files b-g (6 squares)
        assert_eq!(mask.len(), 12);
        assert_eq!(
            mask & Board::FILE_MASKS[0],
            Board::FILE_MASKS[0] ^ Tile::A8 ^ Tile::A1
        );
        assert_eq!(
            mask & Board::RANK_MASKS[0],
            Board::RANK_MASKS[0] ^ Tile::A1 ^ Tile::H1
        );
    }

    #[test]
    fn test_rook_attacks_empty_board() {
        initialize();
        let attacks = magic_moves(Tile::D4, Bitset::empty());

        // On empty board from d4, rook should attack 14 squares
        // (7 on rank + 7 on file)
        assert_eq!(attacks.len(), 14);
    }

    #[test]
    fn test_rook_attacks_blocked() {
        initialize();

        // Place blockers at c4 and e4 (same rank)
        let occupancy = Tile::C4 | Tile::E4;
        let attacks = magic_moves(Tile::D4, occupancy);

        // Rook should attack c4, e4, and all squares on file d
        // Should NOT attack a4, b4, f4, g4, h4
        assert!(attacks.contains(Tile::C4));
        assert!(attacks.contains(Tile::E4));
        assert!(!attacks.contains(Tile::A4));
        assert!(!attacks.contains(Tile::B4));
        assert!(!attacks.contains(Tile::F4));
    }

    #[test]
    fn test_rook_attacks_multiple_blockers() {
        initialize();

        // Place blockers in multiple directions
        let occupancy = Tile::D6 | Tile::D2 | Tile::F4 | Tile::B4;
        let attacks = magic_moves(Tile::D4, occupancy);

        // Should include the blocker squares
        assert!(attacks.contains(Tile::D6));
        assert!(attacks.contains(Tile::D2));
        assert!(attacks.contains(Tile::F4));
        assert!(attacks.contains(Tile::B4));

        // Should not include squares beyond blockers
        assert!(!attacks.contains(Tile::D7));
        assert!(!attacks.contains(Tile::D1));
        assert!(!attacks.contains(Tile::G4));
        assert!(!attacks.contains(Tile::A4));

        // Should include squares between rook and blockers
        assert!(attacks.contains(Tile::D3));
        assert!(attacks.contains(Tile::D5));
        assert!(attacks.contains(Tile::C4));
        assert!(attacks.contains(Tile::E4));
    }

    #[test]
    fn test_rook_corner_attacks() {
        initialize();

        // Test from a1 with no blockers
        let attacks = magic_moves(Tile::A1, Bitset::empty());
        assert_eq!(attacks.len(), 14);

        // Test from h8 with no blockers
        let attacks = magic_moves(Tile::H8, Bitset::empty());
        assert_eq!(attacks.len(), 14);
    }
}
