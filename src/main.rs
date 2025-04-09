use std::fs::File;
use std::io::{self, Read, Write, BufRead, BufReader};
use std::env;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 && args.len() != 2 {
        eprintln!("Usage: {} [tbl2txt|txt2tbl] <input_binary> <output_text>", args[0]);
        std::process::exit(1);
    }

    let mode_flag = &args[1];
    if mode_flag == "--help" || mode_flag == "-h" {
        eprintln!("IronTBL 0.1 by Johan Sjöblom");
        eprintln!("Usage: {} [tbl2txt|txt2tbl] <input_binary> <output_text>", args[0]);
        eprintln!("  tbl2txt: Convert binary *.tbl files to text.");
        eprintln!("  txt2tbl: Convert text to binary *.tbl files.");
        eprintln!("");
        eprintln!("Converts *.tbl files from Blizzard games to text and vice versa.");
        std::process::exit(0);
    }

    let input_path = &args[2];
    let output_path = &args[3];

    if mode_flag == "tbl2txt" {
        read_binary_to_text(input_path, output_path)?;
    } else if mode_flag == "txt2tbl" {
        write_text_to_binary(input_path, output_path)?;
    } else {
        eprintln!("Invalid mode. Use 'tbl2txt' to convert binary to text, 'txt2tbl' to convert text to binary.");
        std::process::exit(1);
    }

    Ok(())
}

fn read_binary_to_text(input_path: &str, output_path: &str) -> io::Result<()> {
    let mut file = File::open(input_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    if buffer.len() < 2 {
        eprintln!("File too small to contain valid data.");
        std::process::exit(1);
    }

    let num_strings = u16::from_le_bytes([buffer[0], buffer[1]]) as usize;
    if buffer.len() < 2 + num_strings * 2 {
        eprintln!("Invalid file format: Not enough data for string offsets.");
        std::process::exit(1);
    }

    let mut offsets = Vec::new();
    for i in 0..num_strings {
        let offset_idx = 2 + i * 2;
        let offset = u16::from_le_bytes([buffer[offset_idx], buffer[offset_idx + 1]]) as usize;
        offsets.push(offset);
    }

    let mut output = File::create(output_path)?;

    for &offset in &offsets {
        if offset >= buffer.len() {
            eprintln!("Invalid string offset detected: {}", offset);
            std::process::exit(1);
        }

        let mut end = offset;
        while end < buffer.len() && buffer[end] != 0 {
            end += 1;
        }

        let string_data = &buffer[offset..end];
        if let Ok(string) = String::from_utf8(string_data.to_vec()) {
            writeln!(output, "{}", string)?;
        } else {
            eprintln!("Invalid UTF-8 sequence at offset {}", offset);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn write_text_to_binary(input_path: &str, output_path: &str) -> io::Result<()> {
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let strings: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    let num_strings = strings.len() as u16;
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&num_strings.to_le_bytes());

    let mut offsets = Vec::new();
    let mut data = Vec::new();
    let mut current_offset = 2 + (num_strings as usize) * 2;

    for string in &strings {
        offsets.push(current_offset as u16);
        let mut bytes = string.as_bytes().to_vec();
        bytes.push(0);
        data.extend_from_slice(&bytes);
        current_offset += bytes.len();
    }

    for offset in offsets {
        buffer.extend_from_slice(&offset.to_le_bytes());
    }

    buffer.extend_from_slice(&data);

    let mut output = File::create(output_path)?;
    output.write_all(&buffer)?;

    Ok(())
}
