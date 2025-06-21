use clap::{Command, CommandFactory, Parser, ValueEnum, ValueHint};
use clap_complete::{generate, Generator, Shell};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, stdout, BufRead, BufReader, Error, ErrorKind, Read, Write};

#[derive(Parser)]
#[command(version, about = "Converts *.tbl files from Blizzard games to text and vice versa.")]
struct Cli {
    #[arg(short, long, value_name = "INPUT_FILE", help = "Specifies the input file path", value_hint = ValueHint::FilePath)]
    input: Option<String>,

    #[arg(short, long, value_name = "OUTPUT_FILE", help = "Specifies the output file path. If omitted in tbl-to-text mode, output goes to stdout.", value_hint = ValueHint::FilePath)]
    output: Option<String>,

    #[arg(short, long, value_name = "MODE", value_enum, help = "Mode of operation")]
    mode: Option<Mode>,

    #[arg(short, long, value_name = "CONTROL_CHAR_MODE", value_enum, help = "Specifies whether to use decimal or hexadecimal for Control characters", default_value_t = ControlCharacterMode::Decimal)]
    control_character_mode: ControlCharacterMode,

    #[arg(short, long, value_name = "LINE_NUMBER", help = "If given, only the specified line will be printed.")]
    line_number: Option<u16>,

    #[arg(long = "generate-shell-completions", value_enum, help = "Generate shell completions")]
    generator: Option<Shell>,
}

#[derive(Clone, ValueEnum)]
enum Mode {
    TblToText,
    TextToTbl,
    Analyse,
}

#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum ControlCharacterMode {
    Decimal,
    Hexadecimal,
}
impl ControlCharacterMode {
    fn radix(self) -> u32 {
        match self {
            ControlCharacterMode::Decimal => 10,
            ControlCharacterMode::Hexadecimal => 16,
        }
    }
}

fn print_completions<G: Generator>(generator: G, cmd: &mut Command) {
    generate(
        generator,
        cmd,
        cmd.get_name().to_string(),
        &mut stdout(),
    );
}

fn main() -> io::Result<()> {
    let args = Cli::parse();
    if let Some(generator) = args.generator {
        let mut cmd = Cli::command();
        println!("Generating completion file for {generator:?}...");
        print_completions(generator, &mut cmd);
        return Ok(());
    }

    if args.mode.is_none() {
        eprintln!("Mode of operation must be specified!");
        std::process::exit(1);
    }
    if args.input.is_none() {
        eprintln!("Input file must be specified!");
        std::process::exit(1);
    }

    match args.mode.unwrap() {
        Mode::TblToText => {
            read_binary_to_text(&args.input.unwrap(), &args.output, &args.control_character_mode, &args.line_number)?;
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
            write_text_to_binary(&args.input.unwrap(), &args.output.unwrap(), &args.control_character_mode)?;
        },
        Mode::Analyse => {
            if args.output.is_some() {
                eprintln!("Output file must not be specified in analyse mode.");
                std::process::exit(1);
            }
            if args.line_number.is_some() {
                eprintln!("Line number option is not applicable in analyse mode.");
                std::process::exit(1);
            }
            analyse(&args.input.unwrap())?;
        },
    }

    Ok(())
}


fn encode_special_bytes(input: &[u8], control_character_mode: &ControlCharacterMode) -> String {
    input.iter().map(|&b| {
        if b < 0x20 || b == 0x3C || b == 0x3E {
            if control_character_mode == &ControlCharacterMode::Decimal {
                format!("<{}>", b)
            } else {
                format!("<{:02X}>", b)
            }
        } else {
            (b as char).to_string()
        }
    }).collect()
}

fn decode_special_strings(input: &str, control_character_mode: &ControlCharacterMode) -> Vec<u8> {
    let mut output = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '<' {
            let mut num_str = String::new();
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == '>' {
                    break;
                }
                num_str.push(next);
            }

            let parsed = u8::from_str_radix(&num_str, control_character_mode.radix());
            if let Ok(num) = parsed {
                output.push(num);
            } else {
                eprintln!("Could not decode control character '{}'. Will drop.", num_str);
            }
        } else {
            output.push(c as u8);
        }
    }
    output
}

fn read_binary_to_text(
    input_path: &str,
    output_path: &Option<String>,
    control_character_mode: &ControlCharacterMode,
    line_number: &Option<u16>,
) -> io::Result<()> {

    let mut file = File::open(input_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    if buffer.len() < 2 {
        eprintln!("File too small to contain valid data.");
        return Err(Error::new(ErrorKind::InvalidInput, "File too small to contain valid data."));
    }

    let num_strings = u16::from_le_bytes([buffer[0], buffer[1]]) as usize;
    if buffer.len() < 2 + num_strings * 2 {
        eprintln!( "Invalid file format: Not enough data for string offsets.");
        return Err(Error::new(ErrorKind::InvalidInput,
              "Invalid file format: Not enough data for string offsets.",
        ));
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

fn write_text_to_binary(
    input_path:  &str,
    output_path: &str,
    control_character_mode: &ControlCharacterMode,
) -> io::Result<()> {

    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let strings: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    let num_strings = strings.len() as u16;
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&num_strings.to_le_bytes());

    let mut offsets = Vec::new();
    let mut data = Vec::new();
    let mut current_offset = 2 + (num_strings as usize) * 2;

    let mut unterminated_strings = HashSet::new();

    for (i, string) in strings.iter().enumerate() {
        if !string.ends_with("<0>") {
            unterminated_strings.insert(i + 1); // 1-based line numbers
        }
        offsets.push(current_offset as u16);
        let bytes = decode_special_strings(string, control_character_mode);
        data.extend_from_slice(&bytes);
        current_offset += bytes.len();
    }

    for offset in offsets {
        buffer.extend_from_slice(&offset.to_le_bytes());
    }

    buffer.extend_from_slice(&data);

    let mut output = File::create(output_path)?;
    output.write_all(&buffer)?;

    if !unterminated_strings.is_empty() {
        let mut lines: Vec<_> = unterminated_strings.into_iter().collect();
        lines.sort();
        println!("Warning: The following lines were not properly null-terminated (missing <0>):\n  {:?}" , lines);
    }

    Ok(())
}

fn analyse(input_path: &str) -> io::Result<()> {
    let mut file = File::open(input_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    if buffer.len() < 2 {
        eprintln!("File too small to contain valid data.");
        return Ok(());
    }

    let num_strings = u16::from_le_bytes([buffer[0], buffer[1]]) as usize;
    println!("Number of entries: {}", num_strings);

    if buffer.len() < 2 + num_strings * 2 {
        println!("File does not contain enough data for all offsets.");
        return Ok(());
    }

    println!("Offset table:");
    let mut offsets = Vec::new();
    for i in 0..num_strings {
        let offset_idx = 2 + i * 2;
        let offset = u16::from_le_bytes([buffer[offset_idx], buffer[offset_idx + 1]]) as usize;
        offsets.push(offset);
        println!("  Entry {:>3}: offset 0x{:0>4X}", i, offset);
    }

    let header_end = 2 + num_strings * 2;
    let first_offset = *offsets.iter().min().unwrap_or(&header_end);

    if first_offset > header_end {
        println!("Warning: Detected {} bytes of unknown data between header and first string:",
                 first_offset - header_end);
        let mut bytes = "".to_string();
        let buf = buffer[header_end..first_offset].to_vec();
        for b in &buf {
            bytes.push_str(&format!("{:02X} ", b));
        }
        println!("{}", &bytes);
    } else {
        println!("No unexpected data detected between header and first string.");
    }

    let mut has_non_null_terminated = false;
    for i in 0..offsets.len() {
        let end = if i + 1 < offsets.len() {
            offsets[i + 1] - 1
        } else {
            buffer.len() - 1
        };
        if end >= buffer.len() {
            println!("  Warning: Offset {} is outside the file bounds.", end);
            continue;
        }
        if buffer[end] != 0 {
            println!("  Warning: Offset {} is not null terminated.", offsets[i]);
            has_non_null_terminated = true;
        }
    }

    if !has_non_null_terminated {
        println!("All strings appear to be correctly null terminated.");
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self};
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_special_byte_encoding_decoding() {
        let mut original = (0..0x20).collect::<Vec<u8>>();
        original.push(0x3C); // '<'
        original.push(0x3E); // '>'

        let encoded = encode_special_bytes( &original, &ControlCharacterMode::Decimal);
        let decoded = decode_special_strings(&encoded, &ControlCharacterMode::Decimal);
        assert_eq!(original, decoded);

        let encoded = encode_special_bytes( &original, &ControlCharacterMode::Hexadecimal);
        let decoded = decode_special_strings(&encoded, &ControlCharacterMode::Hexadecimal);
        assert_eq!(original, decoded);

        let encoded = encode_special_bytes( &original, &ControlCharacterMode::Decimal);
        let decoded = decode_special_strings(&encoded, &ControlCharacterMode::Hexadecimal);
        assert_ne!(original, decoded);
    }

    #[test]
    fn test_text_to_binary_and_back() -> io::Result<()> {
        let ccm = ControlCharacterMode::Decimal;
        let strings = vec![
            "Hello<0><1>World<0>",
            "<2><3><4>Test<0>",
            "String that is not null terminated",
            "String with <60>encoded<62> brackets<0>"
        ];

        // Create temporary input and output files
        let mut input_txt = NamedTempFile::new()?;
        for s in &strings {
            writeln!(input_txt, "{}", s)?;
        }
        let output_bin = NamedTempFile::new()?;
        let result_txt = NamedTempFile::new()?;
        let result_text_path = Some(result_txt.path().to_str().unwrap().to_string());

        write_text_to_binary(input_txt.path().to_str().unwrap(), output_bin.path().to_str().unwrap(), &ccm)?;
        read_binary_to_text(output_bin.path().to_str().unwrap(), &result_text_path, &ccm, &None)?;

        // Compare content
        let result_content = fs::read_to_string(result_txt.path())?;
        let expected_content = strings.join("\n") + "\n"; // writeln! adds newlines
        assert_eq!(result_content, expected_content);

        Ok(())
    }

    #[test]
    fn test_asciitext_to_binary_and_back() -> io::Result<()> {
        let ccm = ControlCharacterMode::Decimal;
        let strings = vec![
            "<72>ello<32>W<111>rld<0>",
        ];

        // Create temporary input and output files
        let mut input_txt = NamedTempFile::new()?;
        for s in &strings {
            writeln!(input_txt, "{}", s)?;
        }
        let output_bin = NamedTempFile::new()?;
        let result_txt = NamedTempFile::new()?;
        let result_text_path = Some(result_txt.path().to_str().unwrap().to_string());

        write_text_to_binary(input_txt.path().to_str().unwrap(), output_bin.path().to_str().unwrap(), &ccm)?;
        read_binary_to_text(output_bin.path().to_str().unwrap(), &result_text_path, &ccm, &None)?;

        // Compare content
        let result_content = fs::read_to_string(result_txt.path())?;
        assert_eq!(result_content, "Hello World<0>\n");

        Ok(())
    }

    #[test]
    fn test_larger_control_characters_are_dropped() -> io::Result<()> {
        let ccm = ControlCharacterMode::Decimal;
        let strings = vec![
            "Hell<366> Worl<355><0>",
        ];

        // Create temporary input and output files
        let mut input_txt = NamedTempFile::new()?;
        for s in &strings {
            writeln!(input_txt, "{}", s)?;
        }
        let output_bin = NamedTempFile::new()?;
        let result_txt = NamedTempFile::new()?;
        let result_text_path = Some(result_txt.path().to_str().unwrap().to_string());

        write_text_to_binary(input_txt.path().to_str().unwrap(), output_bin.path().to_str().unwrap(), &ccm)?;
        read_binary_to_text(output_bin.path().to_str().unwrap(), &result_text_path, &ccm, &None)?;

        // Compare content
        let result_content = fs::read_to_string(result_txt.path())?;
        assert_eq!(result_content, "Hell Worl<0>\n");

        Ok(())
    }
}
