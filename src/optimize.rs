use z3::{Context, Optimize};
use z3::ast::{Ast, Int, Bool};

use crate::Sudoku;

fn add_number_constraints(grid: &Vec<Vec<Int<'_>>>, optimizer: &Optimize, ctx: &Context) {
  let mut number_constraints = Vec::new();
  for i in 0..9 {
      for j in 0..9 {
          number_constraints.push(grid[i][j].ge(&Int::from_u64(ctx, 1)));
          number_constraints.push(grid[i][j].le(&Int::from_u64(ctx, 9)));
      }
  }
  for number_constraint in number_constraints {
      optimizer.assert(&number_constraint);
  }
}

fn add_given_constraints(sudoku: &Sudoku, grid: &Vec<Vec<Int<'_>>>, optimizer: &Optimize, ctx: &Context) {
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
      optimizer.assert(&given_constraint);
  }
}

fn add_horizontal_constraints(grid: &Vec<Vec<Int<'_>>>, optimizer: &Optimize, ctx: &Context) {
  let mut horizontal_constraints = Vec::new();
  for i in 0..9 {
      let mut row = Vec::new();
      for j in 0..9 {
          row.push(&grid[i][j]);
      }
      horizontal_constraints.push(Int::distinct(ctx, &row));
  }
  for horizontal_constraint in horizontal_constraints {
      optimizer.assert(&horizontal_constraint);
  }
}

fn add_vertical_constraints(grid: &Vec<Vec<Int<'_>>>, optimizer: &Optimize, ctx: &Context) {
  let mut vertical_constraints = Vec::new();
  for i in 0..9 {
      let mut col = Vec::new();
      for j in 0..9 {
          col.push(&grid[j][i]);
      }
      vertical_constraints.push(Int::distinct(ctx, &col));
  }
  for vertical_constraint in vertical_constraints {
      optimizer.assert(&vertical_constraint);
  }
}

fn add_nonet_constraints(grid: &Vec<Vec<Int<'_>>>, optimizer: &Optimize, ctx: &Context) {
  let mut nonet_constraints = Vec::new();
  for i in 0..9 {
      let mut nonet = Vec::new();
      for j in 0..9 {
          nonet.push(&grid[((i / 3) * 3) + (j / 3)][((i % 3) * 3) + (j % 3)]);
      }
      nonet_constraints.push(Int::distinct(ctx, &nonet));
  }
  for nonet_constraint in nonet_constraints {
      optimizer.assert(&nonet_constraint);
  }
}

fn add_offset_constraint(grid: &Vec<Vec<Int<'_>>>, offsets: &Vec<Vec<i32>>, optimizer: &Optimize) {
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
      optimizer.assert(&offset_constraint);
  }
}

fn add_increasing_constraint(grid: &Vec<Vec<Int<'_>>>, squares: &Vec<Vec<usize>>, optimizer: &Optimize) {
  let mut increasing_constraints = Vec::new();
  for i in 0..squares.len() - 1 {
      increasing_constraints.push(grid[squares[i][0]][squares[i][1]].lt(&grid[squares[i+1][0]][squares[i+1][1]]));
  }
  for increasing_constraint in increasing_constraints {
      optimizer.assert(&increasing_constraint);
  }
}

fn add_sum_constraint(grid: &Vec<Vec<Int<'_>>>, summands: &[Vec<usize>], sum: &Vec<usize>, optimizer: &Optimize, ctx: &Context) {
  if summands.len() == 0 {
      panic!("No summands found");
  }
  let sum_ast = Int::add(ctx, &summands.iter().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>()[..]);
  optimizer.assert(&grid[sum[0]][sum[1]]._eq(&sum_ast));
}

fn add_exact_diff_constraint(grid: &Vec<Vec<Int<'_>>>, pair: &Vec<Vec<usize>>, diff: u64, optimizer: &Optimize, ctx: &Context) {
  let fst_diff_ast = Int::sub(ctx, &pair.iter().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>());
  let snd_diff_ast = Int::sub(ctx, &pair.iter().rev().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>());
  optimizer.assert(&Bool::or(ctx, &[&fst_diff_ast._eq(&Int::from_u64(ctx, diff)), &snd_diff_ast._eq(&Int::from_u64(ctx, diff))]));
}

fn add_at_least_diff_constraint(grid: &Vec<Vec<Int<'_>>>, pair: &[&Vec<usize>; 2], diff: u64, optimizer: &Optimize, ctx: &Context) {
  let fst_diff_ast = Int::sub(ctx, &pair.iter().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>());
  let snd_diff_ast = Int::sub(ctx, &pair.iter().rev().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>());
  optimizer.assert(&Bool::or(ctx, &[&fst_diff_ast.ge(&Int::from_u64(ctx, diff)), &snd_diff_ast.ge(&Int::from_u64(ctx, diff))]));
}

fn add_kropki_double_constraint(grid: &Vec<Vec<Int<'_>>>, pair: &Vec<Vec<usize>>, optimizer: &Optimize, ctx: &Context) {
  let asts = &pair.iter().map(|x| &grid[x[0]][x[1]]).collect::<Vec<_>>()[..];
  optimizer.assert(
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

pub fn add_optimizer_constraints(sudoku: &Sudoku, grid: &Vec<Vec<Int<'_>>>, optimizer: &Optimize, ctx: &Context) {
  add_number_constraints(grid, optimizer, ctx);
  add_given_constraints(sudoku, grid, optimizer, ctx);
  if sudoku.horizontal_rule {
      add_horizontal_constraints(grid, optimizer, ctx);
  }
  if sudoku.vertical_rule {
      add_vertical_constraints(grid, optimizer, ctx);
  }
  if sudoku.nonet_rule {
      add_nonet_constraints(grid, optimizer, ctx);
  }
  if !sudoku.offset.is_empty() {
      add_offset_constraint(grid, &sudoku.offset, optimizer);
  }
  for squares in &sudoku.thermo {
      add_increasing_constraint(grid, squares, optimizer);
  }
  for squares in &sudoku.arrow {
      add_sum_constraint(grid, &squares[1..], &squares[0], optimizer, ctx);
  }
  for kropki in &sudoku.kropki_adjacent {
      add_exact_diff_constraint(grid, kropki, 1, optimizer, ctx);
  }
  for kropki in &sudoku.kropki_double {
      add_kropki_double_constraint(grid, kropki, optimizer, ctx);
  }
  for whisper in &sudoku.german_whispers {
      for i in 0..whisper.len() - 1 {
          let pair = [&whisper[i], &whisper[i + 1]];
          add_at_least_diff_constraint(grid, &pair, 5, optimizer, ctx);
      }
  }
}
