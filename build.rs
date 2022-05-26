use std::env;
use std::f64;
use std::fmt::Write;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    gen_sin_table(out_dir);
    gen_blackman_harris(out_dir);

    println!("cargo:rerun-if-changed=build.rs");
}

fn gen_sin_table(out_dir: &Path) {
    const LEN: usize = 2048;

    let table = {
        let mut table = [0; LEN];
        for (i, x) in table.iter_mut().enumerate() {
            let sample = f64::sin(2.0 * f64::consts::PI * i as f64 / LEN as f64);
            let fixed_point = (i16::MAX as f64 * sample).round() as i16;
            *x = fixed_point;
        }
        table
    };

    let mut out = String::new();
    writeln!(out, "[").unwrap();
    for x in table {
        writeln!(out, "    {},", x).unwrap();
    }
    writeln!(out, "]").unwrap();

    fs::write(out_dir.join("sin_table.rs"), out).unwrap();
}

fn gen_blackman_harris(out_dir: &Path) {
    const LEN: usize = 1302;

    let table = {
        let mut table = [0; LEN];
        for (i, x) in table.iter_mut().enumerate() {
            let a0 = 0.35875;
            let a1 = 0.48829;
            let a2 = 0.14128;
            let a3 = 0.01168;
            #[rustfmt::skip]
            let sample = a0
                - a1 * f64::cos(2.0 * f64::consts::PI * i as f64 / LEN as f64)
                + a2 * f64::cos(4.0 * f64::consts::PI * i as f64 / LEN as f64)
                - a3 * f64::cos(6.0 * f64::consts::PI * i as f64 / LEN as f64);
            let fixed_point = (u16::MAX as f64 * sample).round() as u16;
            *x = fixed_point;
        }
        table
    };

    let mut out = String::new();
    writeln!(out, "[").unwrap();
    for x in table {
        writeln!(out, "    {},", x).unwrap();
    }
    writeln!(out, "]").unwrap();

    fs::write(out_dir.join("blackman_harris.rs"), out).unwrap();
}
