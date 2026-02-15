use clap::Parser;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::process::{Command, exit};

#[derive(Parser, Debug)]
#[command(name = "env-guard", about = "Validate env vars against a schema")]
struct Args {
    /// Path to the schema file
    #[arg(short, long)]
    schema: String,

    /// Only validate, don't run a command
    #[arg(long)]
    check: bool,

    /// Command and arguments to run after validation
    #[arg(last = true)]
    command: Vec<String>,
}

#[derive(Debug, Clone)]
struct VarSchema {
    name: String,
    required: bool,
    validators: Vec<Validator>,
}

#[derive(Debug, Clone)]
enum Validator {
    Url,
    Integer,
    Range(i64, i64),
    Enum(Vec<String>),
    MinLength(usize),
    Regex(String),
}

fn parse_schema(content: &str) -> Vec<VarSchema> {
    let mut schemas = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, rules)) = line.split_once('=') {
            let parts: Vec<&str> = rules.split(',').collect();
            let required = parts.contains(&"required");
            let mut validators = Vec::new();

            for part in &parts {
                match *part {
                    "url" => validators.push(Validator::Url),
                    "integer" => validators.push(Validator::Integer),
                    p if p.starts_with("range:") => {
                        if let Some(range) = p.strip_prefix("range:") {
                            if let Some((lo, hi)) = range.split_once('-') {
                                if let (Ok(lo), Ok(hi)) = (lo.parse(), hi.parse()) {
                                    validators.push(Validator::Range(lo, hi));
                                }
                            }
                        }
                    }
                    p if p.starts_with("enum:") => {
                        let vals: Vec<String> = p[5..].split('|').map(String::from).collect();
                        validators.push(Validator::Enum(vals));
                    }
                    p if p.starts_with("min_length:") => {
                        if let Ok(n) = p[11..].parse() {
                            validators.push(Validator::MinLength(n));
                        }
                    }
                    p if p.starts_with("regex:") => {
                        validators.push(Validator::Regex(p[6..].to_string()));
                    }
                    _ => {}
                }
            }
            schemas.push(VarSchema {
                name: name.trim().to_string(),
                required,
                validators,
            });
        }
    }
    schemas
}

fn validate(schemas: &[VarSchema]) -> Vec<String> {
    let mut errors = Vec::new();
    let env_vars: HashMap<String, String> = std::env::vars().collect();

    for schema in schemas {
        let value = env_vars.get(&schema.name);

        if schema.required && value.is_none() {
            errors.push(format!("{}: missing (required)", schema.name));
            continue;
        }

        if let Some(val) = value {
            for validator in &schema.validators {
                match validator {
                    Validator::Url => {
                        if url::Url::parse(val).is_err() {
                            errors.push(format!("{}: not a valid URL", schema.name));
                        }
                    }
                    Validator::Integer => {
                        if val.parse::<i64>().is_err() {
                            errors.push(format!("{}: not an integer", schema.name));
                        }
                    }
                    Validator::Range(lo, hi) => {
                        if let Ok(n) = val.parse::<i64>() {
                            if n < *lo || n > *hi {
                                errors.push(format!("{}: {} out of range {}-{}", schema.name, n, lo, hi));
                            }
                        }
                    }
                    Validator::Enum(opts) => {
                        if !opts.contains(&val.to_string()) {
                            errors.push(format!("{}: '{}' not in {:?}", schema.name, val, opts));
                        }
                    }
                    Validator::MinLength(n) => {
                        if val.len() < *n {
                            errors.push(format!("{}: length {} < minimum {}", schema.name, val.len(), n));
                        }
                    }
                    Validator::Regex(pattern) => {
                        if let Ok(re) = regex::Regex::new(pattern) {
                            if !re.is_match(val) {
                                errors.push(format!("{}: doesn't match pattern '{}'", schema.name, pattern));
                            }
                        }
                    }
                }
            }
        }
    }
    errors
}

fn main() {
    let args = Args::parse();
    let schema_content = fs::read_to_string(&args.schema).unwrap_or_else(|e| {
        eprintln!("{} Failed to read schema: {}", "error:".red().bold(), e);
        exit(1);
    });

    let schemas = parse_schema(&schema_content);
    let errors = validate(&schemas);

    if !errors.is_empty() {
        eprintln!("{}", "Environment validation failed:".red().bold());
        for err in &errors {
            eprintln!("  {} {}", "✗".red(), err);
        }
        exit(1);
    }

    println!("{} All {} variables validated", "✓".green().bold(), schemas.len());

    if !args.check && !args.command.is_empty() {
        let status = Command::new(&args.command[0])
            .args(&args.command[1..])
            .status()
            .unwrap_or_else(|e| {
                eprintln!("{} Failed to run command: {}", "error:".red().bold(), e);
                exit(1);
            });
        exit(status.code().unwrap_or(1));
    }
}