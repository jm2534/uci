use crate::game::board::{bitset::Bitset, tile::Tile};
use std::sync::OnceLock;

/// Pre-computed magic numbers for bishop move generation on each square.
/// These numbers were computed via brute force search and are based on
/// well-established values used in production chess engines.
const BISHOP_MAGICS: [u64; 64] = [
    0x40040844404084,
    0x2004208a004208,
    0x10190041080202,
    0x108060845042010,
    0x581104180800210,
    0x2112080446200010,
    0x1080820820060210,
    0x3c0808410220200,
    0x4050404440404,
    0x21001420088,
    0x24d0080801082102,
    0x1020a0a020400,
    0x40308200402,
    0x4011002100800,
    0x401484104104005,
    0x801010402020200,
    0x400210c3880100,
    0x404022024108200,
    0x810018200204102,
    0x4002801a02003,
    0x85040820080400,
    0x810102c808880400,
    0xe900410884800,
    0x8002020480840102,
    0x220200865090201,
    0x2010100a02021202,
    0x152048408022401,
    0x20080002081110,
    0x4001001021004000,
    0x800040400a011002,
    0xe4004081011002,
    0x1c004001012080,
    0x8004200962a00220,
    0x8422100208500202,
    0x2000402200300c08,
    0x8646020080080080,
    0x80020a0200100808,
    0x2010004880111000,
    0x623000a080011400,
    0x42008c0340209202,
    0x209188240001000,
    0x400408a884001800,
    0x110400a6080400,
    0x1840060a44020800,
    0x90080104000041,
    0x201011000808101,
    0x1a2208080504f080,
    0x8012020600211212,
    0x500861011240000,
    0x180806108200800,
    0x4000020e01040044,
    0x300000261044000a,
    0x802241102020002,
    0x20906061210001,
    0x5a84841004010310,
    0x4010801011c04,
    0xa010109502200,
    0x4a02012000,
    0x500201010098b028,
    0x8040002811040900,
    0x28000010020204,
    0x6000020202d0240,
    0x8918844842082200,
    0x4010011029020020,
];

/// Number of relevant occupancy bits for each square.
/// Used to compute the shift amount for magic multiplication.
const BISHOP_SHIFTS: [u8; 64] = [
    6, 5, 5, 5, 5, 5, 5, 6, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 7, 7, 7, 7, 5, 5, 5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5, 5, 5, 7, 7, 7, 7, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 5, 5, 5, 5, 5, 5, 6,
];

/// Global attack table storage. Each square has its own sub-table indexed by
/// the magic hash of the occupancy pattern.
static BISHOP_ATTACKS: OnceLock<Vec<Vec<Bitset>>> = OnceLock::new();

/// Relevant bishop movement patterns for each square, excluding the edge squares
/// in any diagonal direction since, assuming as conventionally done that every blocker is
/// capturable, pieces on the edge don't change the moveset.
const BISHOP_MOVES: [Bitset; 64] = generate_movement_masks();

const fn generate_movement_masks() -> [Bitset; 64] {
    let mut masks = [Bitset::empty(); 64];

    let mut index = 0;
    while index < 64 {
        let tile = Tile::from_index(index);
        let rank = tile.rank() as i32;
        let file = tile.file() as i32;

        let mut mask = Bitset::empty();

        // Northeast diagonal (exclude edges)
        let mut r = rank + 1;
        let mut f = file + 1;
        while r < 7 && f < 7 {
            mask.0 |= Bitset(1 << (r * 8 + f)).0;
            r += 1;
            f += 1;
        }

        // Northwest diagonal (exclude edges)
        let mut r = rank + 1;
        let mut f = file - 1;
        while r < 7 && f > 0 {
            mask.0 |= Bitset(1 << (r * 8 + f)).0;
            r += 1;
            f -= 1;
        }

        // Southeast diagonal (exclude edges)
        let mut r = rank - 1;
        let mut f = file + 1;
        while r > 0 && f < 7 {
            mask.0 |= Bitset(1 << (r * 8 + f)).0;
            r -= 1;
            f += 1;
        }

        // Southwest diagonal (exclude edges)
        let mut r = rank - 1;
        let mut f = file - 1;
        while r > 0 && f > 0 {
            mask.0 |= Bitset(1 << (r * 8 + f)).0;
            r -= 1;
            f -= 1;
        }

        masks[index] = mask;
        index += 1;
    }

    masks
}

/// Initializes the magic bitboard lookup tables.
/// This must be called before using `magic_moves()`.
/// Subsequent calls are no-ops (initialization happens only once).
pub fn initialize() {
    BISHOP_ATTACKS.get_or_init(|| {
        let mut tables = Vec::with_capacity(64);

        // for each tile...
        for tile in (0..64).map(Tile::from_index) {
            // first, get the movement pattern of a piece on this tile
            let movement_mask = BISHOP_MOVES[tile];

            // prepare tile's attack table holding a bitset for each possible blocker pattern
            let relevant_bits = BISHOP_SHIFTS[tile];
            let table_size = 1 << relevant_bits;
            let mut tile_table = vec![Bitset::empty(); table_size];

            // for each occupancy pattern possible from this tile...
            for i in 0..(1 << movement_mask.len()) {
                // populate attack table at magic index with manually generated attacks
                let occupancy = movement_mask.occupancy(i);
                let attacks = generate_bishop_attacks_slow(tile, occupancy);
                let magic_index = magic_index(tile, occupancy);
                tile_table[magic_index] = attacks;
            }

            tables.push(tile_table);
        }

        tables
    });
}

/// Returns the pre-computed bishop attacks for a given tile.
pub(crate) fn magic_moves(tile: Tile, occupancy: Bitset) -> Bitset {
    let index = magic_index(tile, occupancy);
    BISHOP_ATTACKS
        .get()
        .expect("Magic bitboards not initialized - call Board::initialize() first")[tile][index]
}

/// Computes the magic index for a given square and occupancy pattern.
/// This is the core hash function that maps occupancy to attack table indices.
fn magic_index(tile: Tile, occupancy: Bitset) -> usize {
    let mask = BISHOP_MOVES[tile];
    let relevant_occupancy = occupancy & mask;
    let magic = BISHOP_MAGICS[tile];
    let shift = 64 - BISHOP_SHIFTS[tile];
    ((relevant_occupancy.0.wrapping_mul(magic)) >> shift) as usize
}

/// Generates bishop attacks from a square given an occupancy pattern.
/// This is the "slow" version used during initialization.
fn generate_bishop_attacks_slow(tile: Tile, occupancy: Bitset) -> Bitset {
    let square = tile.as_index();
    let rank = (square / 8) as i32;
    let file = (square % 8) as i32;

    let mut attacks = Bitset::empty();

    // Northeast
    let mut r = rank + 1;
    let mut f = file + 1;
    while r < 8 && f < 8 {
        let target = Tile::from_index((r * 8 + f) as usize);
        attacks.insert(target);
        if occupancy.contains(target) {
            break;
        }
        r += 1;
        f += 1;
    }

    // Northwest
    let mut r = rank + 1;
    let mut f = file - 1;
    while r < 8 && f >= 0 {
        let target = Tile::from_index((r * 8 + f) as usize);
        attacks.insert(target);
        if occupancy.contains(target) {
            break;
        }
        r += 1;
        f -= 1;
    }

    // Southeast
    let mut r = rank - 1;
    let mut f = file + 1;
    while r >= 0 && f < 8 {
        let target = Tile::from_index((r * 8 + f) as usize);
        attacks.insert(target);
        if occupancy.contains(target) {
            break;
        }
        r -= 1;
        f += 1;
    }

    // Southwest
    let mut r = rank - 1;
    let mut f = file - 1;
    while r >= 0 && f >= 0 {
        let target = Tile::from_index((r * 8 + f) as usize);
        attacks.insert(target);
        if occupancy.contains(target) {
            break;
        }
        r -= 1;
        f -= 1;
    }

    attacks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        initialize();
        assert!(BISHOP_ATTACKS.get().is_some());
    }

    #[test]
    fn test_bishop_magics() {
        // asserts that all indexed moves are correctly associated
        initialize();
        for tile in (0..64).map(Tile::from_index) {
            let mask = BISHOP_MOVES[tile];
            let attacks = BISHOP_ATTACKS.get().unwrap()[tile].as_slice();

            for i in 0..(1 << mask.len()) {
                let occupancy = mask.occupancy(i);
                let expected = generate_bishop_attacks_slow(tile, occupancy);

                let index = magic_index(tile, occupancy);
                let actual = attacks[index];

                assert_eq!(actual, expected);
            }
        }
    }

    #[test]
    fn test_movement_mask_center() {
        let mask = BISHOP_MOVES[Tile::D4];

        // Center square should have 9 relevant bits (excluding edges on all diagonals)
        // NE: e5, f6, g7 (3)
        // NW: c5, b6 (2)
        // SE: e3, f2 (2)
        // SW: c3, b2 (2)
        assert_eq!(mask.len(), 9);
    }

    #[test]
    fn test_movement_mask_corner() {
        let mask = BISHOP_MOVES[Tile::A1];

        // Corner square should have 6 relevant bits
        // Only one diagonal (NE): b2, c3, d4, e5, f6, g7
        assert_eq!(mask.len(), 6);
    }

    #[test]
    fn test_bishop_attacks_empty_board() {
        initialize();
        let attacks = magic_moves(Tile::D4, Bitset::empty());

        // On empty board from d4, bishop should attack 13 squares
        // NE: 4 squares, NW: 3 squares, SE: 3 squares, SW: 3 squares
        assert_eq!(attacks.len(), 13);
    }

    #[test]
    fn test_bishop_attacks_blocked() {
        initialize();

        // Place blockers at c3 and f6 (on different diagonals)
        let occupancy = Tile::C3 | Tile::F6;
        let attacks = magic_moves(Tile::D4, occupancy);

        // Bishop should attack blocker squares (captures)
        assert!(attacks.contains(Tile::C3));
        assert!(attacks.contains(Tile::F6));

        // Should NOT attack beyond blockers
        assert!(!attacks.contains(Tile::B2));
        assert!(!attacks.contains(Tile::A1));
        assert!(!attacks.contains(Tile::G7));
        assert!(!attacks.contains(Tile::H8));
    }

    #[test]
    fn test_bishop_attacks_multiple_blockers() {
        initialize();

        // Place blockers on all four diagonals
        let occupancy = Tile::F6 | Tile::B6 | Tile::F2 | Tile::B2;
        let attacks = magic_moves(Tile::D4, occupancy);

        // Should include the blocker squares
        assert!(attacks.contains(Tile::F6));
        assert!(attacks.contains(Tile::B6));
        assert!(attacks.contains(Tile::F2));
        assert!(attacks.contains(Tile::B2));

        // Should not include squares beyond blockers
        assert!(!attacks.contains(Tile::G7));
        assert!(!attacks.contains(Tile::A7));
        assert!(!attacks.contains(Tile::G1));
        assert!(!attacks.contains(Tile::A1));

        // Should include squares between bishop and blockers
        assert!(attacks.contains(Tile::E5));
        assert!(attacks.contains(Tile::C5));
        assert!(attacks.contains(Tile::E3));
        assert!(attacks.contains(Tile::C3));
    }

    #[test]
    fn test_bishop_corner_attacks() {
        initialize();

        // Test from a1 with no blockers
        let attacks = magic_moves(Tile::A1, Bitset::empty());
        assert_eq!(attacks.len(), 7); // Only one diagonal

        // Test from h8 with no blockers
        let attacks = magic_moves(Tile::H8, Bitset::empty());
        assert_eq!(attacks.len(), 7);
    }
}
