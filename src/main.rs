mod env_to_yaml;
mod yaml_to_env;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

enum Direction {
    YamlToEnv,
    EnvToYaml,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let mut args = env::args().skip(1);
    let mut direction: Option<Direction> = None;
    let mut input_path: Option<String> = None;
    let mut output_path: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--to" => {
                let target = args
                    .next()
                    .ok_or_else(|| invalid("--to requires a value: env or yaml"))?;
                direction = Some(match target.as_str() {
                    "env" => Direction::YamlToEnv,
                    "yaml" => Direction::EnvToYaml,
                    other => {
                        return Err(invalid(&format!(
                            "unknown target format '{}': expected 'env' or 'yaml'",
                            other
                        )))
                    }
                });
            }
            "-o" | "--output" => {
                output_path = Some(args.next().ok_or_else(|| invalid("--output requires a path"))?);
            }
            other if input_path.is_none() && !other.starts_with('-') => {
                input_path = Some(other.to_string());
            }
            other => return Err(invalid(&format!("unrecognized argument '{}'", other))),
        }
    }

    let direction = direction.ok_or_else(|| invalid("missing --to env|yaml"))?;

    let stdin = io::stdin();
    let reader: Box<dyn BufRead> = match &input_path {
        Some(path) => Box::new(BufReader::new(File::open(path)?)),
        None => Box::new(BufReader::new(stdin.lock())),
    };

    let stdout = io::stdout();
    let writer: Box<dyn Write> = match &output_path {
        Some(path) => Box::new(BufWriter::new(File::create(path)?)),
        None => Box::new(BufWriter::new(stdout.lock())),
    };

    match direction {
        Direction::YamlToEnv => yaml_to_env::convert(reader, writer),
        Direction::EnvToYaml => env_to_yaml::convert(reader, writer),
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.to_string())
}
