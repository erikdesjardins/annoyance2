use std::env;
use std::f64;
use std::fmt::Write;
use std::fs;
use std::path::Path;

const SIN_TABLE_LEN: usize = 1024;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    gen_sin_table(out_dir);

    println!("cargo:rerun-if-changed=build.rs");
}

fn gen_sin_table(out_dir: &Path) {
    let sin_table = {
        let mut sin_table = [0; SIN_TABLE_LEN];
        for (i, x) in sin_table.iter_mut().enumerate() {
            let sin_sample = f64::sin(2.0 * f64::consts::PI * i as f64 / SIN_TABLE_LEN as f64);
            let fixed_point = (i16::MAX as f64 * sin_sample).round() as i16;
            *x = fixed_point;
        }
        sin_table
    };

    let mut out = String::new();
    writeln!(out, "const SIN_TABLE: [i16; {}] = [", SIN_TABLE_LEN).unwrap();
    for x in sin_table {
        writeln!(out, "    {},", x).unwrap();
    }
    writeln!(out, "];").unwrap();

    fs::write(out_dir.join("sin_table.rs"), out).unwrap();
}
