#![no_std]

extern crate alloc;
extern crate ndless_handler;

use alloc::collections::VecDeque;
use ndless::time::Duration;

use ndless::prelude::*;

use ndless::input::{iter_keys, wait_key_pressed, wait_no_key_pressed, Key};
use ndless::process::exit;
use ndless::thread::sleep;
use ndless::time::SystemTime;
use ndless_sdl::nsdl::{Font, FontOptions};

use ndless::msg::{msg_2b, msg_3b, Button};

use ndless_sdl::gfx::framerate::FPS;
use ndless_sdl::video::Surface;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

struct Cell {
    x: i16,
    y: i16,
}

type Mat = [[f32; 4]; 4];

trait MatrixOperationen {
    fn neu_einheit() -> Self;
    fn multipliziere(self, other: &Mat) -> Mat;
    fn transponiere(self) -> Mat;
    fn runde(self, nk_stellen: i32) -> Mat;
}

impl MatrixOperationen for Mat {
    fn neu_einheit() -> Self {
        let mut mat: Mat = Mat::default();
        for (i, element) in mat.iter_mut().enumerate() {
            element[i] = 1.0;
        }

        mat
    }
    // prüft nicht, ob die Matrizen multiplizierbar sind (alle sind quadratisch und von gleicher Größe)
    fn multipliziere(self, other: &Mat) -> Mat {
        let mut res: Mat = Mat::default();
        let n = self.len();

        for i in 0..n {
            for j in 0..n {
                for (k, m2_zeile) in other.iter().enumerate() {
                    res[i][j] += self[i][k] * m2_zeile[j];
                }
            }
        }

        res
    }

    fn transponiere(self) -> Mat {
        let mut transponiert: Mat = Mat::default();

        for (i, zeile) in self.iter().enumerate() {
            for (j, element) in zeile.iter().enumerate() {
                transponiert[j][i] = *element;
            }
        }

        transponiert
    }

    fn runde(self, nk_stellen: i32) -> Mat {
        // die Rundung verändert nicht self, sondern erzeugt eine neue Matrix,
        // damit spätere Rechnungen nicht auf gerundeten Werten basieren
        let mut gerundet = Mat::default();
        let fakt = 10f32.powi(nk_stellen);

        for (i, zeile) in self.iter().enumerate() {
            for (j, element) in zeile.iter().enumerate() {
                // Gleitkommazahlen die 0 sein müssten können sehr kleine positive/negative Werte annehmen
                // durch Rundungsfehler. Eine sehr kleine negative Zahl wird zu -0.0 gerundet,
                // obwohl sie eigentlich 0 sein müsste. aus diesem Grund wird -0.0 zu 0.0 gerundet
                gerundet[i][j] = match (*element * fakt).round() / fakt {
                    f if f == -0.0 => 0.0,
                    f => f,
                };
            }
        }

        gerundet
    }
}

fn main() {
    let a = [
        [1.0, 2.0, 3.0, 4.0],
        [5.0, 6.0, 7.0, 6.0],
        [9.0, 3.0, 11.0, 12.0],
        [12.0, 1.0, 6.0, 7.0],
    ];

    // wird benutzt und verändert bei den Jacobi-Transformationen.
    // a_jacobi ist nach der letzten Iteration bereits Aₕ, aber Aₕ wird danach noch einmal
    // aus A und den U-Matrizen rekonstruiert (ebenso wird analog A rekonstruiert)
    let mut a_jacobi = a;

    print_mat("Matrix A", &a);

    let mut u_matrizen: Vec<Mat> = Vec::new();

    // Finde alle Elemente unterhalb der Nebendiagonale, die nicht 0 sind
    // Alle Indexnamen (hier i0 und j0) enden auf 0, da Elemente ab 0 gezählt werden;
    // nach Konvention ist bspw. i = i0 + 1
    for (iteration, (i0, j0)) in nichtnull_unter_nebendiag(&a).iter().enumerate() {
        println!(
            "Element ({},{}) = {} ≠ 0.",
            i0 + 1,
            j0 + 1,
            a_jacobi[*i0][*j0]
        );
        // Für jedes (i,j), welches 0 werden muss, erfolgt eine Jacobi-Transformation
        // Speichere die U-Matrizen, um sie später für den Test zu verwenden
        let u = jacobi_transform(&mut a_jacobi, *i0, *j0, iteration);
        u_matrizen.push(u);
    }

    // (erneute) Erzeugung der Matrizen Aₕ und A
    // Variable 'acc(umulator)' wird als Einheitsmatrix initialisiert (Mat::neu_einheit());
    // durch "fold" erfolgt dann für jede Matrix "acc = acc * mat"
    let u321_transponiert_multipliziert =
        u_matrizen.iter().rev().fold(Mat::neu_einheit(), |acc, u| {
            acc.multipliziere(&u.transponiere())
        });
    let u123_multipliziert = u_matrizen
        .iter()
        .fold(Mat::neu_einheit(), |acc, u| acc.multipliziere(u));

    // Erzeugung von Aₕ
    let mut ah_rekonstr_titel = String::from("Aₕ = ");
    for i in (0..u_matrizen.len()).rev() {
        ah_rekonstr_titel += &format!("U{}ᵀ * ", i + 1);
    }
    ah_rekonstr_titel += "A";
    for i in 0..u_matrizen.len() {
        ah_rekonstr_titel += &format!(" * U{}", i + 1);
    }
    let ah_rekonstr_mat = u321_transponiert_multipliziert
        .multipliziere(&a)
        .multipliziere(&u123_multipliziert);
    print_mat(&ah_rekonstr_titel, &ah_rekonstr_mat.runde(4));

    // Erzeugung von A
    let mut a_rekonstr_titel = String::from("A = ");
    for i in 0..u_matrizen.len() {
        a_rekonstr_titel += &format!("U{} * ", i + 1);
    }
    a_rekonstr_titel += "Aₕ";
    for i in (0..u_matrizen.len()).rev() {
        a_rekonstr_titel += &format!(" * U{}ᵀ", i + 1);
    }

    let a_rekonstr_mat = u123_multipliziert
        .multipliziere(&ah_rekonstr_mat)
        .multipliziere(&u321_transponiert_multipliziert);
    print_mat(&a_rekonstr_titel, &a_rekonstr_mat.runde(4));
}

fn print_mat(name: &str, mat: &Mat) {
    let hor_strich_breite = mat.len() * 10 - 1;

    // Name gefolgt vom ersten horizontalen Trennstrich
    println!("\x1b[32;1m{name}\x1b[0m:\n {:—<1$}", "", hor_strich_breite);

    for zeile in mat {
        print!("|");
        for element in zeile {
            // gebe jede Gleitkommazahl mit bis zu 7 Stellen aus,
            // mittig in einem 7 Zeichen langen Feld; links und rechts immer ein Leerzeichen.
            print!(" {:^7.7} |", element.to_string());
        }
        // Zeilenumbruch und horizontaler Trennstrich nach jeder Zeile
        println!("\n {:—<1$}", "", hor_strich_breite);
    }
    // Zeilenumbruch nach Matrix
    println!();
}

fn nichtnull_unter_nebendiag(mat: &Mat) -> Vec<(usize, usize)> {
    let mut res = Vec::new();
    let n = mat.len();

    // alle Spalten außer den letzen beiden (da dort i nie > j+1)
    for j0 in 0..n - 2 {
        let i0_range = j0 + 2..n;
        for i0 in i0_range {
            if mat[i0][j0] != 0.0 {
                res.push((i0, j0));
            }
        }
    }

    res
}

fn jacobi_transform(a: &mut Mat, i0: usize, j0: usize, iteration: usize) -> Mat {
    // Rotationsindex
    let (p0, q0) = (j0 + 1, i0);

    let a_jp1j_wert = a[j0 + 1][j0];
    let a_ij_wert = a[i0][j0];

    let (cos_phi, sin_phi) = if a_jp1j_wert == 0.0 {
        (0.0, 1.0)
    } else {
        (
            a_jp1j_wert.abs() / (a_jp1j_wert.powi(2) + a_ij_wert.powi(2)).sqrt(),
            (-(a_jp1j_wert.signum()) * a_ij_wert)
                / (a_jp1j_wert.powi(2) + a_ij_wert.powi(2)).sqrt(),
        )
    };

    println!("Eliminiere mit Φ ≈ {:.3}.", cos_phi.acos());

    let u = ermittle_u(p0, q0, sin_phi, cos_phi);
    print_mat(&format!("U{}", iteration + 1), &u);

    *a = u.transponiere().multipliziere(a).multipliziere(&u);
    u
}

fn ermittle_u(p0: usize, q0: usize, sin_phi: f32, cos_phi: f32) -> Mat {
    let mut u: Mat = Mat::default();

    // u_ii
    for (i, zeile) in u
        .iter_mut()
        .enumerate()
        .filter(|(i, _)| *i != p0 && *i != q0)
    {
        zeile[i] = 1.0;
    }
    // u_pp/u_qq
    u[p0][p0] = cos_phi;
    u[q0][q0] = cos_phi;

    // u_pq
    u[p0][q0] = sin_phi;

    // u_qp
    u[q0][p0] = -sin_phi;

    // u_ij ist bereits 0.0

    u
}


// fn main() {
//     // screen setup
//     let screen = ndless_sdl::init_default().expect("failed to set video mode");

//     // used for score, color indicates difficulty
//     let fonts = vec![
//         // easy
//         Font::new(FontOptions::VGA, 0, 255, 0),
//         // medium
//         Font::new(FontOptions::VGA, 77, 166, 255),
//         // hard
//         Font::new(FontOptions::VGA, 255, 0, 0),
//     ];

//     let mut gradient_calculator = gradient_calculator();

//     let mut manager = FPS::new();
//     manager.framerate(20);

//     // rng setup
//     let mut small_rng = SmallRng::seed_from_u64(
//         SystemTime::now()
//             .duration_since(SystemTime::UNIX_EPOCH)
//             .unwrap()
//             .as_secs(),
//     );

//     // default to medium difficulty. Possible: 0-2
//     let mut difficulty: u8 = 1;

//     // game loop start
//     let mut restart_game = true;
//     while restart_game {
//         clear_screen(&screen);

//         start_game_loop(
//             &screen,
//             &mut manager,
//             &fonts,
//             &mut gradient_calculator,
//             &mut small_rng,
//             &mut difficulty,
//         );
//         restart_game = gameover_handler();
//     }
// }

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
    let mut food_cell = new_food_cell(small_rng, &cells);

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
            if difficulty != &2 {
                sleep(Duration::from_millis(2))
            }
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
        screen.fill_rect(
            Some(ndless_sdl::Rect {
                x: 10,
                y: 10,
                w: 80,
                h: 8,
            }),
            ndless_sdl::video::RGB(0, 0, 0),
        );

        // dont remove oldest vec item if score increased
        if cells.len() > usize::from(length) {
            let delete_cell = cells.pop_front().unwrap();

            screen.fill_rect(
                Some(ndless_sdl::Rect {
                    x: delete_cell.x,
                    y: delete_cell.y,
                    w: 5,
                    h: 5,
                }),
                ndless_sdl::video::RGB(0, 0, 0),
            );
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
            food_cell = new_food_cell(small_rng, &cells);
        }

        screen.flip();
        manager.delay();
    }
}

fn clear_screen(screen: &Surface) {
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

fn new_food_cell(small_rng: &mut SmallRng, cells: &VecDeque<Cell>) -> Cell {
    let mut new_cell = None;

    let mut cell_available = false;
    while !cell_available {
        new_cell = Some(get_random_cell(small_rng));

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

fn get_random_cell(small_rng: &mut SmallRng) -> Cell {
    Cell {
        x: SmallRng::gen_range(small_rng, 0..39) * 5,
        y: SmallRng::gen_range(small_rng, 0..29) * 5,
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
