#![no_std]

extern crate alloc;
extern crate ndless_handler;

use alloc::collections::VecDeque;
use ndless::fs::File;

use ndless::prelude::*;

use ndless::input::{iter_keys, wait_key_pressed, wait_no_key_pressed, Key};
use ndless::process::exit;
use ndless::time::SystemTime;
use ndless_sdl::nsdl::{Font, FontOptions};

use ndless::fs;
use ndless::msg::{msg_2b, msg_3b, Button};

use ndless_sdl::gfx::framerate::FPS;
use ndless_sdl::video::Surface;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

struct Cell {
    x: i16,
    y: i16,
}

fn main() {
    // screen setup
    let screen = ndless_sdl::init_default().expect("failed to set video mode");

    let mut bg_idx = 0;
    let mut background = load_next_background(&mut bg_idx);

    // used for score, color indicates difficulty
    let fonts = vec![
        // easy
        Font::new(FontOptions::VGA, 0, 255, 0),
        // medium
        Font::new(FontOptions::VGA, 77, 166, 255),
        // hard
        Font::new(FontOptions::VGA, 255, 0, 0),
    ];

    let mut gradient_calculator = gradient_calculator();

    let mut manager = FPS::new();
    manager.framerate(20);

    // rng setup
    let mut small_rng = SmallRng::seed_from_u64(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    );

    // default to medium difficulty. Possible: 0-2
    let mut difficulty: u8 = 1;

    // game loop start
    let mut restart_game = true;
    while restart_game {
        clear_screen(&screen, background.as_ref());

        start_game_loop(
            &screen,
            background.as_ref(),
            &mut manager,
            &fonts,
            &mut gradient_calculator,
            &mut small_rng,
            &mut difficulty,
        );
        restart_game = gameover_handler();
    }
}

// float math is way slower, but no problem for this beast of a machine
fn gradient_calculator() -> impl FnMut(usize) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();

    // closure (reuses old vector if length is unchanged)
    move |length: usize| {
        if v.len() != length {
            v = Vec::new();

            let min: u8 = 80;
            let range: u8 = 255 - min;

            let f_min = min as f32;
            let step: f32 = range as f32 / length as f32;

            for i in 0..length - 1 {
                v.push((f_min + i as f32 * step) as u8);
            }
            // head cell is always maxed out
            v.push(255);
        }
        v.clone()
    }
}

fn start_game_loop(
    screen: &Surface,
    background: Option<&Surface>,
    manager: &mut FPS,
    fonts: &[Font],
    mut gradient_calculator: impl FnMut(usize) -> Vec<u8>,
    small_rng: &mut SmallRng,
    difficulty: &mut u8,
) {
    let mut pts: u16 = 0;
    let mut length: u16 = 10;
    let mut cells: VecDeque<Cell> = VecDeque::new();

    // initial spawn location
    cells.push_front(Cell { x: 160, y: 120 });

    let mut mov_direction: u8 = 0; // 0=right 1=down 2=left 3=up

    // first food location
    let mut food_cell = new_food_cell(small_rng, &cells, &difficulty);

    loop {
        let gradients = gradient_calculator(cells.len());
        let mut gradients_iter = gradients.iter();

        let mut event_registered = false;
        for _ in 0..if difficulty == &2 { 1 } else { 2 } {
            for key in iter_keys() {
                if !event_registered {
                    match key {
                        Key::Right => {
                            (mov_direction, event_registered) = get_direction(0, mov_direction)
                        }
                        Key::Key6 => {
                            (mov_direction, event_registered) = get_direction(0, mov_direction);
                        }

                        Key::Down => {
                            (mov_direction, event_registered) = get_direction(1, mov_direction)
                        }
                        Key::Key2 => {
                            (mov_direction, event_registered) = get_direction(1, mov_direction);
                        }

                        Key::Left => {
                            (mov_direction, event_registered) = get_direction(2, mov_direction)
                        }
                        Key::Key4 => {
                            (mov_direction, event_registered) = get_direction(2, mov_direction)
                        }

                        Key::Up => {
                            (mov_direction, event_registered) = get_direction(3, mov_direction)
                        }
                        Key::Key8 => {
                            (mov_direction, event_registered) = get_direction(3, mov_direction)
                        }

                        Key::Scratchpad => pause_game(manager, difficulty),

                        Key::Esc => exit(0),

                        _ => event_registered = false,
                    }
                }
            }
            // if difficulty != &2 {
            //     sleep(Duration::from_millis(2))
            // }
        }

        let mut head = Cell {
            x: cells.back().unwrap().x,
            y: cells.back().unwrap().y,
        };

        if head.x > 315 || head.x < 0 || head.y > 235 || head.y < 0 {
            // player ran into wall: game over - leave game loop
            return;
        }

        // blank score area before redrawing
        let score_area = Some(ndless_sdl::Rect {
            x: 10,
            y: 10,
            w: 80,
            h: 8,
        });
        if let Some(background) = background {
            screen.blit_rect(background, score_area, score_area);
        } else {
            screen.fill_rect(score_area, ndless_sdl::video::RGB(0, 0, 0));
        }

        // dont remove oldest vec item if score increased
        if cells.len() > usize::from(length) {
            let delete_cell = cells.pop_front().unwrap();

            let del_cell_rect = Some(ndless_sdl::Rect {
                x: delete_cell.x,
                y: delete_cell.y,
                w: 5,
                h: 5,
            });

            if let Some(background) = background {
                screen.blit_rect(background, del_cell_rect, del_cell_rect);
            } else {
                screen.fill_rect(del_cell_rect, ndless_sdl::video::RGB(0, 0, 0));
            }
        }

        // draw score
        let message = format!("Punkte: {}", pts);
        screen.draw_str(&fonts[*difficulty as usize], &message, 10, 10);

        for (i, cell) in cells.iter().enumerate() {
            // self hit detection
            if i != cells.len() - 1 && cell.x == head.x && cell.y == head.y {
                // player ran into self: game over - leave game loop
                return;
            }

            let gradient = match gradients_iter.next() {
                Some(res) => *res,
                None => panic!("gradient vector empty"),
            };

            screen.fill_rect(
                Some(ndless_sdl::Rect {
                    x: cell.x,
                    y: cell.y,
                    w: 5,
                    h: 5,
                }),
                ndless_sdl::video::RGB(gradient, gradient, gradient),
            );
        }

        // draw food
        screen.fill_rect(
            Some(ndless_sdl::Rect {
                x: food_cell.x,
                y: food_cell.y,
                w: 5,
                h: 5,
            }),
            ndless_sdl::video::RGB(
                SmallRng::gen_range(small_rng, 100..255),
                SmallRng::gen_range(small_rng, 100..255),
                SmallRng::gen_range(small_rng, 100..255),
            ),
        );

        match mov_direction {
            0 => head.x += 5,
            1 => head.y += 5,
            2 => head.x -= 5,
            3 => head.y -= 5,
            _ => panic!("invalid move direction code"),
        }

        cells.push_back(Cell {
            x: head.x,
            y: head.y,
        });

        if head.x == food_cell.x && head.y == food_cell.y {
            pts += 1;
            length += 2;
            food_cell = new_food_cell(small_rng, &cells, &difficulty);
        }

        screen.flip();
        manager.delay();
    }
}

fn clear_screen(screen: &Surface, background: Option<&Surface>) {
    if let Some(background) = background {
        screen.blit_rect(background, None, None);
    } else {
        screen.fill_rect(
            Some(ndless_sdl::Rect {
                x: 0,
                y: 0,
                w: 320,
                h: 240,
            }),
            ndless_sdl::video::RGB(0, 0, 0),
        );
    }
}

fn get_direction(input: u8, current: u8) -> (u8, bool) {
    match input {
        0 => {
            if current != 2 {
                (0, true)
            } else {
                (2, false)
            }
        }
        1 => {
            if current != 3 {
                (1, true)
            } else {
                (3, false)
            }
        }
        2 => {
            if current != 0 {
                (2, true)
            } else {
                (0, false)
            }
        }
        3 => {
            if current != 1 {
                (3, true)
            } else {
                (1, false)
            }
        }
        _ => panic!("nein"),
    }
}

fn pause_game(manager: &mut FPS, difficulty: &mut u8) {
    // wait for key up (otherwise, still pressed scratchpad key instantly resumes game)
    wait_no_key_pressed();
    loop {
        wait_key_pressed();

        for key in iter_keys() {
            match key {
                Key::Scratchpad => {
                    wait_no_key_pressed();
                    return;
                }
                Key::Enter => {
                    wait_no_key_pressed();
                    difficulty_inp(manager, difficulty);
                    return;
                }
                Key::Esc => exit(0),
                _ => {}
            }
        }
    }
}

fn difficulty_inp(manager: &mut FPS, difficulty: &mut u8) {
    let input = msg_3b(
        "Schwierigkeit",
        "ja der Fensterrahmen ist iwie",
        "noob",
        "ok",
        "weniger ok",
    );

    match input {
        Button::One => {
            manager.framerate(15);
            *difficulty = 0;
        }
        Button::Two => {
            manager.framerate(20);
            *difficulty = 1;
        }
        Button::Three => {
            manager.framerate(30);
            *difficulty = 2;
        }
    }
}

fn new_food_cell(small_rng: &mut SmallRng, cells: &VecDeque<Cell>, difficulty: &u8) -> Cell {
    let mut new_cell = None;

    let mut cell_available = false;
    while !cell_available {
        new_cell = Some(get_random_cell(small_rng, difficulty));

        for snake_cell in cells {
            if snake_cell.x == new_cell.as_ref().unwrap().x
                && snake_cell.y == new_cell.as_ref().unwrap().y
            {
                // food block is on a snake cell -> stop checking other snake cells (and get new food block)
                cell_available = false;
                break;
            }
            // check passed for all snake cells -> food block is in free area
            cell_available = true;
        }
    }

    new_cell.unwrap()
}

fn get_random_cell(small_rng: &mut SmallRng, difficulty: &u8) -> Cell {
    // easy and hard dont get cells directly on the border (hard is too fast for that)
    if *difficulty == 1 {
        Cell {
            x: SmallRng::gen_range(small_rng, 0..39) * 5,
            y: SmallRng::gen_range(small_rng, 0..29) * 5,
        }
    } else {
        Cell {
            x: SmallRng::gen_range(small_rng, 1..38) * 5,
            y: SmallRng::gen_range(small_rng, 1..28) * 5,
        }
    }
}

fn gameover_handler() -> bool {
    let button_pressed = msg_2b(
        "desmond der mondbaer",
        "wie bin ich hier her gekommen?",
        "fail again",
        "rage quit",
    );
    // true if button 1 pressed == game restart requested
    matches!(button_pressed, Button::One)
}

fn load_next_background(bg_idx: &mut u8) -> Option<Surface> {
    let bg_files = fs::read_dir("/documents/backgrounds");
    match bg_files {
        Err(_) => None,
        Ok(dir) => {
            let bg_file = dir.get(*bg_idx as usize).unwrap();
            match bg_file {
                Err(_) => None,
                Ok(f) => ndless_sdl::image::load_file(f).ok(),
            }
        }
    }
    // let bg_file = File::open("/documents/harald.gif.tns");
}
