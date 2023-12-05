use std::{fs::File, io::BufReader};
use clap::{Parser, ValueEnum};
use serde_json::*;
use z3::{Context, SatResult, Solver, Model};
use z3::ast::{Ast, Int, Bool};

#[derive(Debug)]
struct Sudoku {
    given: Vec<Vec<u64>>,
    horizontal_rule: bool,
    vertical_rule: bool,
    nonet_rule: bool,
    offset: Vec<Vec<i32>>,
    thermo: Vec<Vec<Vec<usize>>>,
    arrow: Vec<Vec<Vec<usize>>>,
    kropki_adjacent: Vec<Vec<Vec<usize>>>,
    kropki_double: Vec<Vec<Vec<usize>>>,
    german_whispers: Vec<Vec<Vec<usize>>>
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Mode {
    /// Find a solution of the sudoku
    Solution,

    /// Find the number of solutions of the sudoku (up to MAX_SUDOKU)
    Count,

    /// Find the possible answers in each square
    Hint,

    /// Find the possible answers in a single square
    Square,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File path containing JSON of sudoku
    file_path: String,

    /// What mode to run the solver in
    #[arg(value_enum)]
    mode: Mode,

    /// Use with Square, row of the square to find all possible answers
    row: Option<usize>,

    /// Use with Square, column of the square to find all possible answers
    col: Option<usize>,
}

fn open_sudoku(fp: &String) -> Sudoku {
    let file = File::open(fp).unwrap();
    let reader = BufReader::new(file);
    let v: Value = serde_json::from_reader(reader).unwrap();

    Sudoku {
        given: serde_json::from_value(v["given"].clone()).unwrap(),
        horizontal_rule: serde_json::from_value(v["1-9horiz"].clone()).unwrap(),
        vertical_rule: serde_json::from_value(v["1-9vert"].clone()).unwrap(),
        nonet_rule: serde_json::from_value(v["1-9nonet"].clone()).unwrap(),
        offset: serde_json::from_value(v["offsets"].clone()).unwrap(),
        thermo: serde_json::from_value(v["thermo"].clone()).unwrap(),
        arrow: serde_json::from_value(v["arrow"].clone()).unwrap(),
        kropki_adjacent: serde_json::from_value(v["kropkiAdjacent"].clone()).unwrap(),
        kropki_double: serde_json::from_value(v["kropkiDouble"].clone()).unwrap(),
        german_whispers: serde_json::from_value(v["germanWhispers"].clone()).unwrap(),
    }
}

fn add_number_constraints(grid: &Vec<Vec<Int<'_>>>, solver: &Solver, ctx: &Context) {
    let mut number_constraints = Vec::new();
    for i in 0..9 {
        for j in 0..9 {
            number_constraints.push(grid[i][j].ge(&Int::from_u64(ctx, 1)));
            number_constraints.push(grid[i][j].le(&Int::from_u64(ctx, 9)));
        }
    }
    for number_constraint in number_constraints {
        solver.assert(&number_constraint);
    }
}

fn add_given_constraints(sudoku: &Sudoku, grid: &Vec<Vec<Int<'_>>>, solver: &Solver, ctx: &Context) {
    let mut given_constraints = Vec::new();
    for i in 0..9 {
        for j in 0..9 {
            if sudoku.given[i][j] < 1 || sudoku.given[i][j] > 9 {
                continue;
            }
            given_constraints.push(grid[i][j]._eq(&Int::from_u64(ctx, sudoku.given[i][j])));
        }
    }
    for given_constraint in given_constraints {
        solver.assert(&given_constraint);
    }
}

fn add_horizontal_constraints(grid: &Vec<Vec<Int<'_>>>, solver: &Solver, ctx: &Context) {
    let mut horizontal_constraints = Vec::new();
    for i in 0..9 {
        let mut row = Vec::new();
        for j in 0..9 {
            row.push(&grid[i][j]);
        }
        horizontal_constraints.push(Int::distinct(ctx, &row));
    }
    for horizontal_constraint in horizontal_constraints {
        solver.assert(&horizontal_constraint);
    }
}

fn add_vertical_constraints(grid: &Vec<Vec<Int<'_>>>, solver: &Solver, ctx: &Context) {
    let mut vertical_constraints = Vec::new();
    for i in 0..9 {
        let mut col = Vec::new();
        for j in 0..9 {
            col.push(&grid[j][i]);
        }
        vertical_constraints.push(Int::distinct(ctx, &col));
    }
    for vertical_constraint in vertical_constraints {
        solver.assert(&vertical_constraint);
    }
}

fn add_nonet_constraints(grid: &Vec<Vec<Int<'_>>>, solver: &Solver, ctx: &Context) {
    let mut nonet_constraints = Vec::new();
    for i in 0..9 {
        let mut nonet = Vec::new();
        for j in 0..9 {
            nonet.push(&grid[((i / 3) * 3) + (j / 3)][((i % 3) * 3) + (j % 3)]);
        }
        nonet_constraints.push(Int::distinct(ctx, &nonet));
    }
    for nonet_constraint in nonet_constraints {
        solver.assert(&nonet_constraint);
    }
}

fn add_offset_constraint(grid: &Vec<Vec<Int<'_>>>, offsets: &Vec<Vec<i32>>, solver: &Solver) {
    let mut offset_constraints = Vec::new();
    for i in 0..9 {
        for j in 0..9 {
            let squares = offsets.iter().map(|x| ((i as i32) + x[0], (j as i32) + x[1])).filter(|(a, b)| 0 <= *a && *a < 9 && 0 <= *b && *b < 9);
            for (row, col) in squares {
                offset_constraints.push(Bool::not(&grid[i][j]._eq(&grid[row as usize][col as usize])));
            }
        }
    }
    for offset_constraint in offset_constraints {
        solver.assert(&offset_constraint);
    }
}

fn add_increasing_constraint(grid: &Vec<Vec<Int<'_>>>, squares: &Vec<Vec<usize>>, solver: &Solver) {
    let mut increasing_constraints = Vec::new();
    for i in 0..squares.len() - 1 {
        increasing_constraints.push(grid[squares[i][0]][squares[i][1]].lt(&grid[squares[i+1][0]][squares[i+1][1]]));
    }
    for increasing_constraint in increasing_constraints {
        solver.assert(&increasing_constraint);
    }
}

fn add_sum_constraint(grid: &Vec<Vec<Int<'_>>>, summands: &[Vec<usize>], sum: &Vec<usize>, solver: &Solver, ctx: &Context) {
    if summands.len() == 0 {
        panic!("No summands found");
    }
    let sum_ast = Int::add(ctx, &summands.iter().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>()[..]);
    solver.assert(&grid[sum[0]][sum[1]]._eq(&sum_ast));
}

fn add_exact_diff_constraint(grid: &Vec<Vec<Int<'_>>>, pair: &Vec<Vec<usize>>, diff: u64, solver: &Solver, ctx: &Context) {
    let fst_diff_ast = Int::sub(ctx, &pair.iter().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>());
    let snd_diff_ast = Int::sub(ctx, &pair.iter().rev().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>());
    solver.assert(&Bool::or(ctx, &[&fst_diff_ast._eq(&Int::from_u64(ctx, diff)), &snd_diff_ast._eq(&Int::from_u64(ctx, diff))]));
}

fn add_at_least_diff_constraint(grid: &Vec<Vec<Int<'_>>>, pair: &[&Vec<usize>; 2], diff: u64, solver: &Solver, ctx: &Context) {
    let fst_diff_ast = Int::sub(ctx, &pair.iter().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>());
    let snd_diff_ast = Int::sub(ctx, &pair.iter().rev().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>());
    solver.assert(&Bool::or(ctx, &[&fst_diff_ast.ge(&Int::from_u64(ctx, diff)), &snd_diff_ast.ge(&Int::from_u64(ctx, diff))]));
}

fn add_kropki_double_constraint(grid: &Vec<Vec<Int<'_>>>, pair: &Vec<Vec<usize>>, solver: &Solver, ctx: &Context) {
    let asts = &pair.iter().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>()[..];
    solver.assert(
        &Bool::or(ctx,
            &[
                &Bool::and(ctx, &[&asts[0]._eq(&Int::from_u64(ctx, 1)), &asts[1]._eq(&Int::from_u64(ctx, 2))]),
                &Bool::and(ctx, &[&asts[0]._eq(&Int::from_u64(ctx, 2)), &asts[1]._eq(&Int::from_u64(ctx, 1))]),
                &Bool::and(ctx, &[&asts[0]._eq(&Int::from_u64(ctx, 2)), &asts[1]._eq(&Int::from_u64(ctx, 4))]),
                &Bool::and(ctx, &[&asts[0]._eq(&Int::from_u64(ctx, 3)), &asts[1]._eq(&Int::from_u64(ctx, 6))]),
                &Bool::and(ctx, &[&asts[0]._eq(&Int::from_u64(ctx, 4)), &asts[1]._eq(&Int::from_u64(ctx, 2))]),
                &Bool::and(ctx, &[&asts[0]._eq(&Int::from_u64(ctx, 4)), &asts[1]._eq(&Int::from_u64(ctx, 8))]),
                &Bool::and(ctx, &[&asts[0]._eq(&Int::from_u64(ctx, 6)), &asts[1]._eq(&Int::from_u64(ctx, 3))]),
                &Bool::and(ctx, &[&asts[0]._eq(&Int::from_u64(ctx, 8)), &asts[1]._eq(&Int::from_u64(ctx, 4))]),
            ]
        )
    );
}

fn add_constraints(sudoku: &Sudoku, grid: &Vec<Vec<Int<'_>>>, solver: &Solver, ctx: &Context) {
    add_number_constraints(grid, solver, ctx);
    add_given_constraints(sudoku, grid, solver, ctx);
    if sudoku.horizontal_rule {
        add_horizontal_constraints(grid, solver, ctx);
    }
    if sudoku.vertical_rule {
        add_vertical_constraints(grid, solver, ctx);
    }
    if sudoku.nonet_rule {
        add_nonet_constraints(grid, solver, ctx);
    }
    if !sudoku.offset.is_empty() {
        add_offset_constraint(grid, &sudoku.offset, solver);
    }
    for squares in &sudoku.thermo {
        add_increasing_constraint(grid, squares, solver);
    }
    for squares in &sudoku.arrow {
        add_sum_constraint(grid, &squares[1..], &squares[0], solver, ctx);
    }
    for kropki in &sudoku.kropki_adjacent {
        add_exact_diff_constraint(grid, kropki, 1, solver, ctx);
    }
    for kropki in &sudoku.kropki_double {
        add_kropki_double_constraint(grid, kropki, solver, ctx);
    }
    for whisper in &sudoku.german_whispers {
        for i in 0..whisper.len() - 1 {
            let pair = [&whisper[i], &whisper[i + 1]];
            add_at_least_diff_constraint(grid, &pair, 5, solver, ctx);
        }
    }
}

fn print_sudoku_from_model(model: &Model, grid: &Vec<Vec<Int<'_>>>) {
    let mut sudoku = [[0; 9]; 9];
    for i in 0..9 {
        for j in 0..9 {
            sudoku[i][j] =  model.get_const_interp(&grid[i][j]).unwrap().as_u64().unwrap();
        }
    }
    println!("╔═══════╤═══════╤═══════╗");
    for i in 0..9 {
        print!("║");
        for j in 0..3 {
            print!(" {} {} {} ", sudoku[i][3 * j], sudoku[i][3 * j + 1], sudoku[i][3 * j + 2]);
            if j != 2 {
                print!("│");
            }
        }
        println!("║");
        if i % 3 == 2 && i != 8 {
            println!("╟───────┼───────┼───────╢")
        }
    }
    println!("╚═══════╧═══════╧═══════╝");
}

fn main() {
    let args = Args::parse();

    let sudoku = open_sudoku(&args.file_path);

    let config = z3::Config::new();
    let ctx = z3::Context::new(&config);
    let solver = Solver::new(&ctx);

    let grid = (0..9).map(|i: i32| (0..9).map(|j| Int::new_const(&ctx, format!("r{i}c{j}"))).collect()).collect::<Vec<Vec<_>>>();

    match args.mode {
        Mode::Solution => {
            if args.row.is_some() || args.col.is_some() {
                println!("Ignoring row and column information in Solution mode.");
            }
            add_constraints(&sudoku, &grid, &solver, &ctx);
            println!("Constraints added. Solver is running...");
            match solver.check() {
                SatResult::Sat => {
                    println!("Possible solution found!");
                    let model = solver.get_model().unwrap();
                    print_sudoku_from_model(&model, &grid);
                },
                SatResult::Unsat => {
                    println!("Could not find a satisfying Sudoku");
                },
                SatResult::Unknown => {
                    panic!("Solver returned unknown!");
                }
            }
        },
        Mode::Count => (),
        Mode::Hint => (),
        Mode::Square => {
            if args.row.is_none() || args.col.is_none() {
                println!("Please specify the row and column of the square.");
                return;
            }
            let row = args.row.unwrap();
            let col = args.col.unwrap();
            if 9 <= row || 9 <= col {
                println!("Invalid square, {} {}", row, col);
                return;
            }
            add_constraints(&sudoku, &grid, &solver, &ctx);
            println!("Constraints added. Solver is running...");
            for i in 1..=9 {
                print!("Checking {}... ", i);
                solver.push();
                solver.assert(&grid[row][col]._eq(&Int::from_u64(&ctx, i)));
                if solver.check() == SatResult::Sat {
                    println!("True!");
                } else {
                    println!("False!");
                }
                solver.pop(1);
            }
        }
    }
}
