extern crate clap;

use clap::Parser;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::process;
use std::{fs, io};

#[derive(Parser, Debug)]
struct Args {
    input_bytecode: std::path::PathBuf,
    output_rs: std::path::PathBuf,
    variable_name: String,
}

fn main() {
    let args = Args::parse();
    println!("Input file {:?}", &args.input_bytecode);
    println!("Ouput file {:?}", &args.output_rs);

    let bytecode = BufReader::new(File::open(args.input_bytecode).unwrap());

    let string_template = format!(
        "#[repr(align(4))]\npub struct AlignedBytes(pub [u8; {}]);\npub const wrapper: AlignedBytes = AlignedBytes([\n{}\n]);\npub const {}: &[u8] = &wrapper.0;\n",
        bytecode.get_ref().metadata().expect("").len(),
        bytecode
            .bytes()
            .map(|b| format!("0x{:02x}, ", b.unwrap()))
            .collect::<String>(),
        args.variable_name
    );

    fs::write(&args.output_rs, string_template).unwrap_or_else(|e| {
        eprintln!("Failed to write {}: {}", args.output_rs.display(), e);
        process::exit(1);
    });
}
