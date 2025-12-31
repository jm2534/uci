use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use uci::engine::strategy::Strategy;
use uci::engine::strategy::minimax::Minimax;
use uci::game::Color;
use uci::game::board::Board;
use uci::game::moves::PseudoLegalMove;

// Test positions for benchmarking
struct TestPosition {
    name: &'static str,
    fen: &'static str,
}

const TEST_POSITIONS: &[TestPosition] = &[
    TestPosition {
        name: "starting",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    },
    TestPosition {
        name: "midgame_open",
        fen: "r1bqk2r/pp2bppp/2n1pn2/3p4/2PP4/2N2NP1/PP2PPBP/R1BQK2R w KQkq - 0 8",
    },
    TestPosition {
        name: "midgame_closed",
        fen: "rnbqkb1r/pp2pppp/5n2/2pp4/3P4/2N2N2/PPP1PPPP/R1BQKB1R w KQkq - 0 5",
    },
    TestPosition {
        name: "tactical_complex",
        fen: "r1bq1rk1/ppp2ppp/2np1n2/2b1p3/2B1P3/2NP1N2/PPP2PPP/R1BQ1RK1 w - - 0 8",
    },
    TestPosition {
        name: "endgame_rook",
        fen: "8/5k2/8/5P2/8/3R4/8/6K1 w - - 0 1",
    },
];

/// Benchmark legal move generation (includes legality checking via make-and-test)
/// This measures the cost of full legal move validation including board cloning.
/// Throughput reports moves/second - higher values mean faster move generation.
/// This is significantly slower than pseudo-legal due to make/unmake overhead.
fn bench_legal_movegen(c: &mut Criterion) {
    Board::initialize();

    let mut group = c.benchmark_group("legal_movegen");

    for position in TEST_POSITIONS {
        let mut board = Board::try_from(position.fen).unwrap();

        // Pre-calculate number of legal moves for throughput reporting (moves/second)
        let mut moves_count = Vec::new();
        board.populate_legal_moves(board.to_move(), &mut moves_count);
        let num_moves = moves_count.len();
        group.throughput(Throughput::Elements(num_moves as u64));

        let moves = Vec::new();
        group.bench_with_input(
            BenchmarkId::from_parameter(position.name),
            &board,
            move |b, board| {
                b.iter_batched_ref(
                    || {
                        let board = board.clone();
                        let mut moves = moves.clone();
                        moves.clear();
                        (board, moves)
                    },
                    |(board, moves)| {
                        // Collect moves to force iterator evaluation and legality checking
                        let color = board.to_move();
                        black_box(board).populate_legal_moves(color, moves);
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark try_move (make move on board)
/// This measures the cost of executing a move including all state updates.
/// Throughput reports moves/second (1 move per iteration).
/// Useful for understanding the overhead of the make-and-test approach.
fn bench_make_move(c: &mut Criterion) {
    Board::initialize();

    let mut group = c.benchmark_group("make_move");

    for position in TEST_POSITIONS {
        let mut board = Board::try_from(position.fen).unwrap();

        // Get first legal move for this position
        let mut moves = Vec::new();
        board.populate_legal_moves(board.to_move(), &mut moves);
        let first_move = moves.first().copied();
        if let Some(first_move) = first_move {
            // Throughput is 1 move per iteration (measures individual move execution speed)
            group.throughput(Throughput::Elements(1));

            group.bench_with_input(
                BenchmarkId::from_parameter(position.name),
                &(board, first_move),
                |b, (board, move_to_make)| {
                    b.iter_batched_ref(
                        || board.to_owned(),
                        |board| {
                            black_box(board).make_validated_move(*move_to_make).unwrap();
                        },
                        criterion::BatchSize::SmallInput,
                    );
                },
            );
        }
    }

    group.finish();
}

/// Benchmark move ordering (capture prioritization)
/// This measures the cost of sorting moves for better alpha-beta pruning.
fn bench_move_ordering(c: &mut Criterion) {
    Board::initialize();

    let mut group = c.benchmark_group("move_ordering");

    for position in TEST_POSITIONS {
        let board = Board::try_from(position.fen).unwrap();
        let mut moves = Vec::with_capacity(32);
        let strategy = Minimax::default();

        group.bench_with_input(
            BenchmarkId::from_parameter(position.name),
            &board,
            |b, board| {
                b.iter_batched_ref(
                    || {
                        moves.clear();
                        (board.clone(), moves.clone())
                    },
                    |(board, moves)| black_box(&strategy).order(board, moves),
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark board evaluation function
/// This measures the cost of material counting and position evaluation.
/// Pure evaluation without any search overhead.
fn bench_evaluation(c: &mut Criterion) {
    Board::initialize();

    let mut group = c.benchmark_group("evaluation");

    for position in TEST_POSITIONS {
        let board = Board::try_from(position.fen).unwrap();

        let strategy = Minimax::default();

        group.bench_with_input(
            BenchmarkId::from_parameter(position.name),
            &board,
            |b, board| {
                b.iter(|| {
                    let score = black_box(&strategy).evaluate(black_box(board));
                    black_box(score)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark search at fixed depth (no time limit)
/// This is the most important benchmark for understanding overall engine performance.
/// Throughput reports nodes/second (NPS) - the standard chess engine performance metric.
/// Each "node" represents one call to the evaluation function or terminal position check.
/// Higher NPS indicates faster search, though quality also depends on pruning effectiveness.
fn bench_search_depth(c: &mut Criterion) {
    Board::initialize();

    let mut group = c.benchmark_group("search_fixed_depth");
    group.sample_size(10); // Reduce sample size since search is expensive

    let depths = [3, 4, 5, 6]; // Test multiple depths

    let mut buffers = vec![Vec::with_capacity(256); *depths.last().unwrap()];
    for depth in depths {
        for position in TEST_POSITIONS.iter().take(3) {
            // Only test first 3 positions to save time
            let mut board = Board::try_from(position.fen).unwrap();

            // Pre-calculate node count for throughput reporting
            // This gives us nodes/second (NPS) - the standard chess engine performance metric
            let (_, nodes) = search_to_depth(&mut board, depth, &mut buffers[..depth]);
            group.throughput(Throughput::Elements(nodes as u64));

            group.bench_with_input(
                BenchmarkId::new(position.name, depth),
                &board,
                move |b, board| {
                    b.iter_batched_ref(
                        || {
                            let board = board.to_owned();
                            assert_eq!(
                                board.undo_stack.len(),
                                0,
                                "Fresh board should have empty undo_stack"
                            );
                            let buffers = vec![Vec::with_capacity(256); depth];
                            (board, buffers)
                        },
                        |(board, buffers)| search_to_depth(board, depth, &mut buffers[..depth]),
                        BatchSize::SmallInput,
                    );
                },
            );
        }
    }

    group.finish();
}

/// Helper function for depth-limited search
/// Returns (best_move, nodes_searched)
/// This is what you'd typically measure for "nodes per second" in chess engines.
fn search_to_depth(
    board: &mut Board,
    depth: usize,
    buffers: &mut [Vec<PseudoLegalMove>],
) -> (Option<uci::game::moves::PseudoLegalMove>, usize) {
    let strategy = Minimax::default();

    let alpha = isize::MIN;
    let beta = isize::MAX;

    match board.to_move() {
        Color::Black => {
            let (result, nodes) = strategy.minvalue(board, depth, alpha, beta, buffers);
            (result.attempt, nodes)
        }
        Color::White => {
            let (result, nodes) = strategy.maxvalue(board, depth, alpha, beta, buffers);
            (result.attempt, nodes)
        }
    }
}

criterion_group!(board_benches, bench_legal_movegen, bench_make_move);

criterion_group!(
    search_benches,
    bench_move_ordering,
    bench_evaluation,
    bench_search_depth,
);

criterion_main!(board_benches, search_benches);
