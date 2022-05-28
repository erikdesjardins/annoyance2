use std::env;
use std::f64;
use std::fmt::{Display, Write};
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    gen_adc_sin_table(out_dir);
    gen_fft_sin_table(out_dir);
    gen_hamming(out_dir);
    gen_blackman_nutall(out_dir);
    gen_blackman_harris(out_dir);

    println!("cargo:rerun-if-changed=build.rs");
}

fn gen_adc_sin_table(out_dir: &Path) {
    write_sin_table::<633>(&out_dir.join("adc_sin_table.rs"));
}

fn gen_fft_sin_table(out_dir: &Path) {
    write_sin_table::<2048>(&out_dir.join("fft_sin_table.rs"));
}

fn gen_hamming(out_dir: &Path) {
    write_window_coefficients(&out_dir.join("hamming.rs"), [0.53836, 0.46164, 0., 0.]);
}

fn gen_blackman_nutall(out_dir: &Path) {
    write_window_coefficients(
        &out_dir.join("blackman_nutall.rs"),
        [0.3635819, 0.4891775, 0.1365995, 0.0106411],
    );
}

fn gen_blackman_harris(out_dir: &Path) {
    write_window_coefficients(
        &out_dir.join("blackman_harris.rs"),
        [0.35875, 0.48829, 0.14128, 0.01168],
    );
}

fn write_sin_table<const LEN: usize>(file_path: &Path) {
    let table = {
        let mut table = [0; LEN];
        for (i, x) in table.iter_mut().enumerate() {
            let sample = f64::sin(2.0 * f64::consts::PI * i as f64 / LEN as f64);
            let fixed_point = (i16::MAX as f64 * sample).round() as i16;
            *x = fixed_point;
        }
        table
    };

    write_table(file_path, &table)
}

fn write_window_coefficients(file_path: &Path, a: [f64; 4]) {
    const LEN: usize = 633;

    let table = {
        let mut table = [0; LEN];
        for (i, x) in table.iter_mut().enumerate() {
            #[rustfmt::skip]
            let sample = a[0]
                - a[1] * f64::cos(2.0 * f64::consts::PI * i as f64 / LEN as f64)
                + a[2] * f64::cos(4.0 * f64::consts::PI * i as f64 / LEN as f64)
                - a[3] * f64::cos(6.0 * f64::consts::PI * i as f64 / LEN as f64);
            let fixed_point = (u16::MAX as f64 * sample).round() as u16;
            *x = fixed_point;
        }
        table
    };

    write_table(file_path, &table);
}

fn write_table<T>(file_path: &Path, table: &[T])
where
    T: Display + NumericSuffix,
{
    let mut out = String::new();

    out.push('[');
    let mut first = true;
    for x in table {
        write!(out, "{}", x).unwrap();
        if first {
            first = false;
            // add type suffix to first element to ensure we don't accidentally use the wrong type
            out.push_str(T::SUFFIX);
        }
        out.push_str(",\n");
    }
    out.push(']');

    fs::write(file_path, out).unwrap();
}

trait NumericSuffix {
    const SUFFIX: &'static str;
}

impl NumericSuffix for u16 {
    const SUFFIX: &'static str = "u16";
}

impl NumericSuffix for i16 {
    const SUFFIX: &'static str = "i16";
}
