use std::fs::File;
use std::io::{self, stdout, Read, Write, BufRead, BufReader};
use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(version, about = "Converts *.tbl files from Blizzard games to text and vice versa.")]
struct Cli {
    #[arg(short, long, value_name = "INPUT_FILE", help = "Specifies the input file path")]
    input: String,

    #[arg(short, long, value_name = "OUTPUT_FILE", help = "Specifies the output file path. If omitted in read mode, output goes to stdout.")]
    output: Option<String>,

    #[arg(short, long, value_name = "MODE", value_enum, help = "Mode of operation")]
    mode: Mode,

    #[arg(short, long, value_name = "LINE_NUMBER", help = "If given, only the specified line will be printed.")]
    line_number: Option<u16>,
}

#[derive(Clone, ValueEnum)]
enum Mode {
    TblToText,
    TextToTbl,
}


fn main() -> io::Result<()> {
    let args = Cli::parse();

    match args.mode {
        Mode::TblToText => {
            read_binary_to_text(&args.input, &args.output, &args.line_number)?;
        },
        Mode::TextToTbl => {
            if args.output.is_none() {
                eprintln!("Output file must be specified in text-to-tbl mode.");
                std::process::exit(1);
            }
            if args.line_number.is_some() {
                eprintln!("Line number option is not applicable in text-to-tbl mode.");
                std::process::exit(1);
            }
            write_text_to_binary(&args.input, &args.output.unwrap())?;
        },
    }

    Ok(())
}


fn read_binary_to_text(
    input_path: &str,
    output_path: &Option<String>,
    line_number: &Option<u16>,
) -> io::Result<()> {

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

    let mut output: Box<dyn Write> = match output_path {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(stdout()),
    };

    for i in 0..offsets.len() {
        if line_number.is_some() && line_number != &Some(i as u16) {
            continue;
        }

        let start = offsets[i];
        let end = if i + 1 < offsets.len() {
            offsets[i + 1]
        } else {
            buffer.len()
        };

        let string_data = &buffer[start..end];
        let encoded = encode_special_bytes(string_data, control_character_mode);
        writeln!(output, "{}", encoded)?;
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
