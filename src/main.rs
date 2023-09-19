#![no_std]

extern crate alloc;
extern crate ndless_handler;

use alloc::collections::VecDeque;
use ndless::time::Duration;

use ndless::prelude::*;

use ndless::input::{iter_keys, wait_key_pressed, wait_no_key_pressed, Key};
use ndless::process::exit;
use ndless::thread::sleep;
use ndless_sdl::nsdl::Font;

use ndless::msg::{msg_2b, msg_3b, Button};

use ndless_sdl::gfx::framerate::FPS;
use ndless_sdl::video::Surface;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};


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
    let mut mess = String::new();
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

    mess += &print_mat("Matrix A", &a);

    let mut u_matrizen: Vec<Mat> = Vec::new();

    // Finde alle Elemente unterhalb der Nebendiagonale, die nicht 0 sind
    // Alle Indexnamen (hier i0 und j0) enden auf 0, da Elemente ab 0 gezählt werden;
    // nach Konvention ist bspw. i = i0 + 1
    for (iteration, (i0, j0)) in nichtnull_unter_nebendiag(&a).iter().enumerate() {
        mess += &format!(
            "Element ({},{}) = {} ≠ 0.\n",
            i0 + 1,
            j0 + 1,
            a_jacobi[*i0][*j0]
        );
        // Für jedes (i,j), welches 0 werden muss, erfolgt eine Jacobi-Transformation
        // Speichere die U-Matrizen, um sie später für den Test zu verwenden
        let u = jacobi_transform(&mut a_jacobi, *i0, *j0, iteration, &mut mess);
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

fn print_mat(name: &str, mat: &Mat) -> String {
    let str = String::new();
    let hor_strich_breite = mat.len() * 10 - 1;

    // Name gefolgt vom ersten horizontalen Trennstrich
    str += &format!("{name}\n {:—<1$}", "", hor_strich_breite);


    for zeile in mat {
        str += "|";
        for element in zeile {
            // gebe jede Gleitkommazahl mit bis zu 7 Stellen aus,
            // mittig in einem 7 Zeichen langen Feld; links und rechts immer ein Leerzeichen.
            str += &format!(" {:^7.7} |", element.to_string());
        }
        // Zeilenumbruch und horizontaler Trennstrich nach jeder Zeile
        str += !format!("\n {:—<1$}\n", "", hor_strich_breite);
    }
    // Zeilenumbruch nach Matrix
    str += "\n";

    str
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

fn jacobi_transform(a: &mut Mat, i0: usize, j0: usize, iteration: usize, mess: &mut String) -> Mat {
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

    *mess += &format!("Eliminiere mit Φ ≈ {:.3}.\n", cos_phi.acos());

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
