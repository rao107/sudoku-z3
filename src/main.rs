mod solver;
mod optimize;

use std::{fs::File, io::BufReader};
use clap::{Parser, ValueEnum};
use serde_json::*;
use z3::{SatResult, Solver, Model, Optimize};
use z3::ast::{Ast, Int, Bool};

use crate::solver::add_solver_constraints;
use crate::optimize::add_optimizer_constraints;

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

    /// Find the number of solutions of the sudoku (up to max_sudoku)
    Count,

    /// Find the possible answers in each square
    Hint,

    /// Find the possible answers in a single square
    Square,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File path containing JSON of Sudoku
    #[arg(short, long)]
    file_path: String,

    /// What mode to run the solver in
    #[arg(long, value_enum)]
    mode: Mode,

    /// Maximum number of Sudokus to search
    #[arg(long, default_value_t = 1000)]
    max_sudoku: u32,

    /// Use with Square, row of the square to find all possible answers
    #[arg(short, long)]
    row: Option<usize>,

    /// Use with Square, column of the square to find all possible answers
    #[arg(short, long)]
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

    let grid = (0..9).map(|i: i32| (0..9).map(|j| Int::new_const(&ctx, format!("r{i}c{j}"))).collect()).collect::<Vec<Vec<_>>>();

    match args.mode {
        Mode::Solution => {
            if args.row.is_some() || args.col.is_some() {
                println!("Ignoring row and column information in Solution mode.");
            }
            let solver = Solver::new(&ctx);
            add_solver_constraints(&sudoku, &grid, &solver, &ctx);
            println!("Constraints added. Solver is running...");
            match solver.check() {
                SatResult::Sat => {
                    println!("Possible solution found!");
                    let model = solver.get_model().unwrap();
                    print_sudoku_from_model(&model, &grid);
                },
                SatResult::Unsat => {
                    println!("Could not find a satisfying Sudoku.");
                },
                SatResult::Unknown => {
                    panic!("Solver returned unknown!");
                }
            }
        },
        Mode::Count => {
            if args.row.is_some() || args.col.is_some() {
                println!("Ignoring row and column information in Solution mode.");
            }
            let solver = Solver::new(&ctx);
            add_solver_constraints(&sudoku, &grid, &solver, &ctx);
            println!("Constraints added. Counting solutions...");
            for num in 0..args.max_sudoku {
                match solver.check() {
                    SatResult::Sat => {
                        let model = solver.get_model().unwrap();
                        let mut filled_sudoku = [[0; 9]; 9];
                        for i in 0..9 {
                            for j in 0..9 {
                                filled_sudoku[i][j] = model.get_const_interp(&grid[i][j]).unwrap().as_u64().unwrap();
                            }
                        }
                        let a = grid.iter().enumerate().flat_map(
                            |(i, x)| x.iter().enumerate().map(
                                |(j, y)| Bool::not(&y._eq(&Int::from_u64(&ctx, filled_sudoku[i][j])))
                            ).collect::<Vec<_>>()
                        ).collect::<Vec<_>>();
                        solver.assert(&Bool::or(&ctx, &a.iter().map(|x| x).collect::<Vec<_>>()[..]));
                    }
                    SatResult::Unsat => {
                        println!("Found {num} possible sudokus!");
                        return;
                    }
                    SatResult::Unknown => {
                        println!("Unknown reached? Stopping...");
                        return;
                    }
                }
            }
            println!("Found >{} possible sudokus!", args.max_sudoku);
        },
        Mode::Hint => {
            if args.row.is_some() || args.col.is_some() {
                println!("Ignoring row and column information in Solution mode.");
            }
            let optimizer = Optimize::new(&ctx);
            add_optimizer_constraints(&sudoku, &grid, &optimizer, &ctx);
            let mut clues = [[[false; 9]; 9]; 9];
            println!("Constraints added. Finding all possible values of every square...");
            for num in 1..=args.max_sudoku {
                match optimizer.check(&[]) {
                    SatResult::Sat => {
                        let model = optimizer.get_model().unwrap();
                        let mut answer = [[0; 9]; 9];
                        let mut new_info = 0;
                        for i in 0..9 {
                            for j in 0..9 {
                                answer[i][j] = model.get_const_interp(&grid[i][j]).unwrap().as_u64().unwrap();
                                if !clues[i][j][(answer[i][j] - 1) as usize] {
                                    new_info += 1;
                                    clues[i][j][(answer[i][j] - 1) as usize] = true;
                                }
                            }
                        }
                        println!("Iteration {num}: Found {new_info} new clues");
                        if new_info == 0 {
                            for i in 0..9 {
                                for j in 0..9 {
                                    print!("Row {i} Column {j}: ");
                                    for k in 0..9 {
                                        if clues[i][j][k] {
                                            print!("{} ", k + 1);
                                        }
                                    }
                                    println!();
                                }
                            }
                            return;
                        }
                        for i in 0..9 {
                            for j in 0..9 {
                                optimizer.assert_soft(&Bool::not(&grid[i][j]._eq(&Int::from_u64(&ctx, answer[i][j]))), 1, None);
                            }
                        }
                    }
                    SatResult::Unsat => {
                        println!("Could not find a satisfying sudoku.");
                        return;
                    }
                    SatResult::Unknown => {
                        println!("Unknown reached? Stopping...");
                        return;
                    }
                }
            }
            println!("Reached maximum iterations ({}). Try adding more constraints or increase max_sudoku.", args.max_sudoku);
            println!("Known hints found so far:");
            for i in 0..9 {
                for j in 0..9 {
                    print!("Row {i} Column {j}: ");
                    for k in 0..9 {
                        if clues[i][j][k] {
                            print!("{} ", k + 1);
                        }
                    }
                    println!();
                }
            }
        },
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
            let solver = Solver::new(&ctx);
            add_solver_constraints(&sudoku, &grid, &solver, &ctx);
            println!("Constraints added. Finding possible values...");
            for i in 1..=9 {
                println!("Checking {}...", i);
                solver.push();
                solver.assert(&grid[row][col]._eq(&Int::from_u64(&ctx, i)));
                match solver.check() {
                    SatResult::Sat => println!("True!"),
                    SatResult::Unsat => println!("False!"),
                    SatResult::Unknown => println!("Unknown!"),
                }
                solver.pop(1);
            }
        }
    }
}
