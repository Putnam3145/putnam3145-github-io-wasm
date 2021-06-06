mod utils;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct Material {
    name: String,
    solidDensity: f64,
    impactYield: f64,
    impactFracture: f64,
    shearYield: f64,
    shearFracture: f64,
    maxEdge: f64,
    armor: bool,
}
#[derive(Serialize, Deserialize)]
pub struct Attack {
    name: String,
    edged: bool,
    velocity: f64,
    area: f64,
}

#[derive(Serialize, Deserialize)]
pub struct Weapon {
    name: String,
    size: f64,
    attacks: Vec<Attack>,
}
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_size(u: usize);
}

const QUALITY_WEAPON_MULTS: [f64; 6] = [0.5, 0.6, 0.7, 0.8, 0.9, 1.0];

const QUALITY_ARMOR_MULTS: [f64; 7] = [1.0, 1.2, 1.4, 1.6, 1.8, 2.0, 3.0];

const BODY_SIZE_BUCKETS: [f64; 7] = [0.9, 0.95, 0.98, 1.0, 1.02, 1.05, 1.10];

const DWARF_STRENGTH_BUCKETS: [i32; 7] = [450, 950, 1150, 1250, 1350, 1550, 2250];

use rand::{rngs::ThreadRng, thread_rng, Rng};

fn bucket_random<T: rand::distributions::uniform::SampleUniform + PartialOrd + Copy>(
    arr: &[T],
    rng: &mut ThreadRng,
) -> T {
    let bucket = rng.gen_range(0..arr.len() - 1);
    rng.gen_range(arr[bucket]..arr[bucket + 1])
}

#[allow(non_snake_case)]
#[wasm_bindgen]
pub fn attack_score(
    attack_js: &JsValue,
    weapon_mat_js: &JsValue,
    armor_mat_js: &JsValue,
    weapon_js: &JsValue,
    weapon_weight: f64,
) -> f64 {
    utils::set_panic_hook();
    let attack: Attack = attack_js.into_serde().unwrap();
    let weapon_mat: Material = weapon_mat_js.into_serde().unwrap();
    let armor_mat: Material = armor_mat_js.into_serde().unwrap();
    let weapon: Weapon = weapon_js.into_serde().unwrap();
    let mut trials = 0;
    let mut rng = thread_rng();
    let successes = [0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 3, 3, 4, 4, 5, 5, 6]
        .iter()
        .map(|&a| {
            let Qa = QUALITY_ARMOR_MULTS[a as usize];
            [0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5]
                .iter()
                .map(|&w| {
                    let Qw = QUALITY_WEAPON_MULTS[w as usize];
                    let size = 60000.0
                        * (0..3).fold(1.0, |acc, _| {
                            acc * bucket_random(&BODY_SIZE_BUCKETS, &mut rng)
                        });
                    let enemy_size = 60000.0
                        * (0..3).fold(1.0, |acc, _| {
                            acc * bucket_random(&BODY_SIZE_BUCKETS, &mut rng)
                        });
                    let strength = bucket_random(&DWARF_STRENGTH_BUCKETS, &mut rng) as f64;
                    let momentum = size * strength * attack.velocity
                        / (10.0 * ((10000.0 + size) / weapon_weight));
                    [0.036, 0.0027]
                        .iter()
                        .filter(|&&contact_area| {
                            // breastplate, helmet
                            let area = attack.area.min(enemy_size * contact_area);
                            trials += 1;
                            (attack.edged
                                && momentum
                                    >= ((armor_mat.shearYield / weapon_mat.shearYield)
                                        + ((area + 1.0)
                                            * (armor_mat.shearFracture
                                                / weapon_mat.shearFracture))
                                            * ((10.0 + 2.0 * Qa)
                                                / (Qw * weapon_mat.maxEdge / 10000.0))))
                                || ((2.0 * weapon.size * weapon_mat.impactYield / 1000.0
                                    >= area * armor_mat.solidDensity)
                                    && (momentum
                                        >= (((2.0 * armor_mat.impactFracture) / 1000000.0
                                            - armor_mat.impactYield / 1000000.0)
                                            * (2.0 + 0.4 * Qa)
                                            * area)))
                        })
                        .count()
                })
                .sum::<usize>()
        })
        .sum::<usize>();
    (100.0 * successes as f64 / trials as f64).round() / 10.0
}

struct Adjacents {
    index: usize,
    width: usize,
    size: usize,
    i: usize,
}

impl Iterator for Adjacents {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.i += 1;
            match self.i {
                1 => {
                    if self.index % self.width != 0 {
                        return Some(self.index - 1);
                    }
                }
                2 => {
                    if self.index % self.width != (self.width - 1) {
                        return Some(self.index + 1);
                    }
                }
                3 => {
                    if self.index >= self.width {
                        return Some(self.index - self.width);
                    }
                }
                4 => {
                    if self.index + self.width < self.size {
                        return Some(self.index + self.width);
                    }
                }
                _ => return None,
            }
        }
    }
}

fn adjacent_tiles(index: usize, width: usize, size: usize) -> Adjacents {
    Adjacents {
        index,
        width,
        size,
        i: 0,
    }
}

#[wasm_bindgen]
pub fn fdm(arr: &[f64], width: usize) -> Box<[f64]> {
    arr.iter()
        .enumerate()
        .map(|(i, f)| {
            let (num_adj, adjacent_gas) = adjacent_tiles(i, width, arr.len())
                .fold((0, 0.0), |acc, j| { (acc.0 + 1, acc.1 + arr[j] / 5.0) });
            (f * (5-num_adj) as f64) / 5.0 + adjacent_gas
        })
        .collect::<Vec<f64>>()
        .into_boxed_slice()
}
