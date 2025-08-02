use std::ops::Mul;

//use ::rand::prelude::*;
use ::rand::{Rng, rng};
use macroquad::prelude::*;
//use std::arch::x86_64;

static COLOUR_ARRAY: [Color; 9] = [WHITE, BLUE, GREEN, RED, PURPLE, ORANGE, YELLOW, BLACK, PINK];

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum TileContents {
    Bomb,
    Clear(u32),
}

#[derive(Copy, Clone, Debug)]
struct TileState {
    ishidden: bool,
    contents: TileContents,
}

impl Default for TileState {
    fn default() -> Self {
        Self {
            ishidden: true,
            contents: TileContents::Clear(0),
        }
    }
}

impl TileState {
    fn draw_tile(&self, x: f32, y: f32, length: f32) {
        /* Improve how this looks */
        match self.ishidden {
            true => draw_rectangle(x + 1.0, y + 1.0, length - 2.0, length - 2.0, GRAY),
            false => draw_rectangle(x + 1.0, y + 1.0, length - 2.0, length - 2.0, LIGHTGRAY),
        }

        match (self.ishidden, self.contents) {
            (true, _) => draw_text("", 0.0, 0.0, 0.0, BLACK),
            (false, TileContents::Bomb) => {
                draw_text("B", x + length / 4.0, y + length * 0.75, length, RED)
            }
            (false, TileContents::Clear(n)) => draw_text(
                format!("{n}").as_str(),
                x + length / 4.0,
                y + length * 0.75,
                length,
                COLOUR_ARRAY[n as usize],
            ),
        };
    }

    fn show(&mut self) {
        self.ishidden = false;
    }

    fn is_zero(&self) -> bool {
        match self.contents {
            TileContents::Clear(0) => true,
            _ => false,
        }
    }

    fn is_bomb(&self) -> bool {
        match self.contents {
            TileContents::Bomb => true,
            _ => false,
        }
    }
}

fn generate_game(mine_count: u32, grid_size: usize) -> Vec<Vec<TileState>> {
    let mut arr = vec![vec![TileState::default(); grid_size]; grid_size];
    let mut generator = rng();

    let mut bombs_placed: u32 = 0;
    while bombs_placed < mine_count {
        let x = generator.random::<u32>() % grid_size as u32;
        let y = generator.random::<u32>() % grid_size as u32;

        //println!("{:?}, {:?}", x, y);

        match &mut arr[x as usize][y as usize].contents {
            TileContents::Bomb => continue,
            t => {
                *t = TileContents::Bomb;

                bombs_placed += 1;

                // This is a rather disgusting loop saying increment the numbers of neighbouring notes

                for (u, v) in offsets(x as _, y as _) {
                    if u >= 0 && u < grid_size as i32 {
                        if v >= 0 && v < grid_size as i32 {
                            match &mut arr[u as usize][v as usize].contents {
                                TileContents::Bomb => (),
                                TileContents::Clear(n) => {
                                    *n += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    arr
}

fn offsets(x: i32, y: i32) -> impl Iterator<Item = (i32, i32)> {
    (x - 1..=x + 1).flat_map(move |x| (y - 1..=y + 1).map(move |y| (x, y)))
}

fn show_tile(array: &mut [Vec<TileState>], x: u32, y: u32) {
    array[x as usize][y as usize].show();
    //println!(array);

    if array[x as usize][y as usize].is_zero() {
        for (u, v) in offsets(x as _, y as _) {
            if u >= 0 && u < array.len() as i32 {
                if v >= 0 && v < array[u as usize].len() as i32 {
                    if array[u as usize][v as usize].ishidden {
                        //println!("{:?} {:?}", u, v);
                        show_tile(array, u as u32, v as u32);
                    }
                }
            }
        }
    }
}

fn square<T: Mul<T, Output = T> + Copy>(x: T) -> T {
    x * x
}

// MINESWEEPER TASKS
// 1. Get the grid displaying:
// - Requirements: B text if bomb, number if clear.
// How to do this:
// impl Tilestate - add methods to your object.
// have a function that takes self called like `fn draw(&self, position stuff)` that will draw it
// When it comes to deciding how to draw a number and or bomb icon, you need to extract state from your tuple:
// match self{
//   Self::Bomb => {draw bomb}
//   Self::Clear(count) => {draw number}
//}

// 2. Get the grid randomly generatoring
// This will require a random number generator. there is a package called `rand`, google it
// You will need to mutate your array, so use iter_mut and declar the array as let mut to allow changes
// You will want two states - one that randomly places bombs, one that fills in the clears

// 3. Make a unit test for the part that fills in clears. Make it test a small (3x3) grid that you manually make,
// And test that the correct clears are made
//  - you will need the concept of equality to test your tile state (==). This can be done with deriving PartialEq, Eq.
//   - (partial eq is for things like floats where self == self is not always true)

#[macroquad::main("MyGame")]
async fn main() {
    const GRID_SIZE: usize = 10;

    let grid = &mut generate_game(17, GRID_SIZE);

    //show_tile(grid, 4, 1);

    debug!("{}", square(2));

    loop {
        let squaresize = screen_width().min(screen_height()) / GRID_SIZE as f32;
        let topleftx;
        let toplefty;
        if screen_height() > screen_width() {
            (topleftx, toplefty) = (
                0.0,
                screen_height() / 2.0 - squaresize * GRID_SIZE as f32 / 2.0,
            );
        } else {
            (topleftx, toplefty) = (
                screen_width() / 2.0 - squaresize * GRID_SIZE as f32 / 2.0,
                0.0,
            )
        }

        if is_mouse_button_released(MouseButton::Left) {
            let (mousex, mousey) = mouse_position();
            let (diffx, diffy) = (
                (mousex - topleftx) / squaresize as f32,
                (mousey - toplefty) / squaresize as f32,
            );
            println!("{:?} {:?}", diffx, diffy);

            show_tile(grid, diffx as u32, diffy as u32);

            if grid[diffx as usize][diffy as usize].is_bomb() {
                // break;
            }
        }

        clear_background(DARKGRAY);

        for (x, column) in grid.iter().enumerate() {
            for (y, tile) in column.iter().enumerate() {
                tile.draw_tile(
                    topleftx + x as f32 * squaresize,
                    toplefty + y as f32 * squaresize,
                    squaresize,
                );
            }
        }

        next_frame().await
    }
}
