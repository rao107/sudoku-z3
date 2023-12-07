# sudoku-z3

A Command-Line Interface for solving and setting sudokus using [Z3 Rust bindings](https://github.com/prove-rs/z3.rs).

Use [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) to build this project using `cargo build --release`.

This project is a companion to the [Javascript sudoku setter](https://github.com/rao107/sudoku-setter) created for CS 560, Reasoning About Programs. To obtain valid JSON files to input into this CLI consider using the setter or use the provided [blank template](./sudoku-export.json).

Supports the following variants:
* Thermo
* Arrow
* Kropki
* German Whispers
* Anti-knight
  - Add `"offsets": [[-1, -1], [-1, 0], [-1, 1], [0, -1], [0, 1], [1, -1], [1, 0], [1, 1]]`
* Anti-king
  - Add `"offsets": [[-2, -1], [-2, 1], [-1, -2], [-1, 2], [1, -2], [1, 2], [2, -1], [2, 1]]`

Can also disable normal sudoku rules (distinct numbers in every row, column, and nonet).

## Solution Mode

Solution mode will find a single solution to a given Sudoku puzzle.

Example usage:

```
$ ./target/release/sudoku-z3 -f ./sudoku-export.json --mode solution
Constraints added. Solver is running...
Possible solution found!
╔═══════╤═══════╤═══════╗
║ 8 6 4 │ 7 2 9 │ 5 3 1 ║
║ 9 1 2 │ 4 5 3 │ 7 6 8 ║
║ 3 7 5 │ 6 1 8 │ 2 4 9 ║
╟───────┼───────┼───────╢
║ 6 4 9 │ 8 7 5 │ 3 1 2 ║
║ 7 2 1 │ 9 3 6 │ 8 5 4 ║
║ 5 3 8 │ 2 4 1 │ 6 9 7 ║
╟───────┼───────┼───────╢
║ 4 8 6 │ 5 9 7 │ 1 2 3 ║
║ 1 9 7 │ 3 6 2 │ 4 8 5 ║
║ 2 5 3 │ 1 8 4 │ 9 7 6 ║
╚═══════╧═══════╧═══════╝
```

## Count Mode

Count mode will enumerate how many ways a given Sudoku can be solved and print them. By default, it will stop after counting 1,000 Sudokus. This can be changed by passing another value for `max_sudoku`.

**Note:** This mode does not work well with Sudokus with few constraints. Its intended purpose is to ensure a given Sudoku has only one solution.

Example usage:

```
$ ./target/debug/sudoku-z3 -f ./sudoku-export.json --mode count --max-sudoku 1
Constraints added. Counting solutions...
╔═══════╤═══════╤═══════╗
║ 8 6 4 │ 7 2 9 │ 5 3 1 ║
║ 9 1 2 │ 4 5 3 │ 7 6 8 ║
║ 3 7 5 │ 6 1 8 │ 2 4 9 ║
╟───────┼───────┼───────╢
║ 6 4 9 │ 8 7 5 │ 3 1 2 ║
║ 7 2 1 │ 9 3 6 │ 8 5 4 ║
║ 5 3 8 │ 2 4 1 │ 6 9 7 ║
╟───────┼───────┼───────╢
║ 4 8 6 │ 5 9 7 │ 1 2 3 ║
║ 1 9 7 │ 3 6 2 │ 4 8 5 ║
║ 2 5 3 │ 1 8 4 │ 9 7 6 ║
╚═══════╧═══════╧═══════╝
Found >1 possible sudokus!
```

## Hint Mode

Hint mode will find all possible numbers that can fill each square.

**Note:** This mode may become slow if the Sudoku does not have enough constraints like an empty grid. If this happens, consider switching to square mode for squares of most importance.

Example usage:

```
$ ./target/release/sudoku-z3 -f ./sudoku-export.json --mode hint
Constraints added. Finding all possible values of every square...
Iteration 1: Found 81 new clues
Iteration 2: Found 4 new clues
Iteration 3: Found 0 new clues
Row 0 Column 0: 8 
Row 0 Column 1: 6 
Row 0 Column 2: 4 
Row 0 Column 3: 7 
Row 0 Column 4: 2
...
```

## Square Mode

Square mode will find all possible numbers that can fill a single square.

Example usage:

```
$ ./target/release/sudoku-z3 -f ./sudoku-export.json --mode square -r 0 -c 0
Constraints added. Finding possible values...
Checking 1...
True!
Checking 2...
True!
Checking 3...
True!
Checking 4...
True!
Checking 5...
True!
Checking 6...
True!
Checking 7...
True!
Checking 8...
True!
Checking 9...
True!
```
