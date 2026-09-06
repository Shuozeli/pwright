use std::env;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use pwright_schema_gen::{
    check_minor_compatibility_with_imports, generate_capability_rust_with_imports,
    generate_markdown_reference_with_imports, generate_rust_with_imports,
    generate_server_rust_with_imports, parse_and_validate, parse_and_validate_with_imports,
};

#[derive(Debug)]
enum CliError {
    Usage,
    DuplicateArgument(&'static str),
    MissingValue(&'static str),
    UnknownArgument(String),
    PreviousImportWithoutPrevious,
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Schema(pwright_schema_gen::SchemaError),
    Compatibility(pwright_schema_gen::CompatibilityError),
    Drift {
        paths: Vec<PathBuf>,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl Display for CliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => formatter.write_str(
                "usage: pwright-schema-gen --input <schema.yml> --output <generated.rs> [--capability-output <generated.rs>] [--server-output <generated.rs>] [--reference-output <generated.md>] [--import <library.yml>]... [--previous <published.yml> [--previous-import <published-library.yml>]...] [--check]",
            ),
            Self::DuplicateArgument(argument) => {
                write!(formatter, "argument `{argument}` was repeated")
            }
            Self::MissingValue(argument) => {
                write!(formatter, "argument `{argument}` needs a value")
            }
            Self::UnknownArgument(argument) => write!(formatter, "unknown argument `{argument}`"),
            Self::PreviousImportWithoutPrevious => {
                formatter.write_str("`--previous-import` requires `--previous`")
            }
            Self::Read { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
            Self::Schema(error) => Display::fmt(error, formatter),
            Self::Compatibility(error) => Display::fmt(error, formatter),
            Self::Drift { paths } => {
                formatter.write_str("generated output is missing or stale:")?;
                for path in paths {
                    write!(formatter, " {}", path.display())?;
                }
                formatter.write_str("; run pwright-schema-gen without --check")
            }
            Self::Write { path, source } => {
                write!(formatter, "could not write {}: {source}", path.display())
            }
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Write { source, .. } => Some(source),
            Self::Schema(error) => Some(error),
            Self::Compatibility(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Arguments {
    input: PathBuf,
    output: PathBuf,
    capability_output: Option<PathBuf>,
    server_output: Option<PathBuf>,
    reference_output: Option<PathBuf>,
    previous: Option<PathBuf>,
    imports: Vec<PathBuf>,
    previous_imports: Vec<PathBuf>,
    check: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), CliError> {
    let arguments = parse_arguments(env::args().skip(1))?;
    let source = fs::read_to_string(&arguments.input).map_err(|source| CliError::Read {
        path: arguments.input.clone(),
        source,
    })?;
    let imports = read_schemas(&arguments.imports)?;
    let schema = parse_and_validate_with_imports(&source, &imports).map_err(CliError::Schema)?;
    if let Some(previous_path) = &arguments.previous {
        let previous_source =
            fs::read_to_string(previous_path).map_err(|source| CliError::Read {
                path: previous_path.clone(),
                source,
            })?;
        let previous_imports = if arguments.previous_imports.is_empty() {
            imports.clone()
        } else {
            read_schemas(&arguments.previous_imports)?
        };
        let previous = parse_and_validate_with_imports(&previous_source, &previous_imports)
            .map_err(CliError::Schema)?;
        check_minor_compatibility_with_imports(&previous, &schema, &previous_imports, &imports)
            .map_err(CliError::Compatibility)?;
    }
    let generated = generate_rust_with_imports(&schema, &imports).map_err(CliError::Schema)?;
    let generated_capability = arguments
        .capability_output
        .as_ref()
        .map(|_| generate_capability_rust_with_imports(&schema, &imports))
        .transpose()
        .map_err(CliError::Schema)?;
    let generated_server = arguments
        .server_output
        .as_ref()
        .map(|_| generate_server_rust_with_imports(&schema, &imports))
        .transpose()
        .map_err(CliError::Schema)?;
    let generated_reference = arguments
        .reference_output
        .as_ref()
        .map(|_| generate_markdown_reference_with_imports(&schema, &imports))
        .transpose()
        .map_err(CliError::Schema)?;
    if arguments.check {
        let mut stale = Vec::new();
        if fs::read_to_string(&arguments.output).ok().as_deref() != Some(generated.as_str()) {
            stale.push(arguments.output.clone());
        }
        if let (Some(path), Some(generated)) = (
            arguments.capability_output.as_ref(),
            generated_capability.as_ref(),
        ) && fs::read_to_string(path).ok().as_deref() != Some(generated.as_str())
        {
            stale.push(path.clone());
        }
        if let (Some(path), Some(generated)) =
            (arguments.server_output.as_ref(), generated_server.as_ref())
            && fs::read_to_string(path).ok().as_deref() != Some(generated.as_str())
        {
            stale.push(path.clone());
        }
        if let (Some(path), Some(generated)) = (
            arguments.reference_output.as_ref(),
            generated_reference.as_ref(),
        ) && fs::read_to_string(path).ok().as_deref() != Some(generated.as_str())
        {
            stale.push(path.clone());
        }
        if !stale.is_empty() {
            return Err(CliError::Drift { paths: stale });
        }
        return Ok(());
    }
    fs::write(&arguments.output, generated).map_err(|source| CliError::Write {
        path: arguments.output,
        source,
    })?;
    if let (Some(path), Some(generated)) = (arguments.capability_output, generated_capability) {
        fs::write(&path, generated).map_err(|source| CliError::Write { path, source })?;
    }
    if let (Some(path), Some(generated)) = (arguments.server_output, generated_server) {
        fs::write(&path, generated).map_err(|source| CliError::Write { path, source })?;
    }
    if let (Some(path), Some(generated)) = (arguments.reference_output, generated_reference) {
        fs::write(&path, generated).map_err(|source| CliError::Write { path, source })?;
    }
    Ok(())
}

fn read_schemas(paths: &[PathBuf]) -> Result<Vec<pwright_schema_gen::ProtocolSchema>, CliError> {
    paths
        .iter()
        .map(|path| {
            let source = fs::read_to_string(path).map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })?;
            parse_and_validate(&source).map_err(CliError::Schema)
        })
        .collect()
}

fn parse_arguments(arguments: impl Iterator<Item = String>) -> Result<Arguments, CliError> {
    let mut input = None;
    let mut output = None;
    let mut capability_output = None;
    let mut server_output = None;
    let mut reference_output = None;
    let mut previous = None;
    let mut imports = Vec::new();
    let mut previous_imports = Vec::new();
    let mut check = false;
    let mut arguments = arguments.peekable();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--input" => parse_path(&mut input, "--input", &mut arguments)?,
            "--output" => parse_path(&mut output, "--output", &mut arguments)?,
            "--capability-output" => parse_path(
                &mut capability_output,
                "--capability-output",
                &mut arguments,
            )?,
            "--server-output" => parse_path(&mut server_output, "--server-output", &mut arguments)?,
            "--reference-output" => {
                parse_path(&mut reference_output, "--reference-output", &mut arguments)?
            }
            "--previous" => parse_path(&mut previous, "--previous", &mut arguments)?,
            "--import" => imports.push(PathBuf::from(
                arguments.next().ok_or(CliError::MissingValue("--import"))?,
            )),
            "--previous-import" => previous_imports.push(PathBuf::from(
                arguments
                    .next()
                    .ok_or(CliError::MissingValue("--previous-import"))?,
            )),
            "--check" if check => return Err(CliError::DuplicateArgument("--check")),
            "--check" => check = true,
            _ => return Err(CliError::UnknownArgument(argument)),
        }
    }
    if previous.is_none() && !previous_imports.is_empty() {
        return Err(CliError::PreviousImportWithoutPrevious);
    }
    Ok(Arguments {
        input: input.ok_or(CliError::Usage)?,
        output: output.ok_or(CliError::Usage)?,
        capability_output,
        server_output,
        reference_output,
        previous,
        imports,
        previous_imports,
        check,
    })
}

fn parse_path(
    slot: &mut Option<PathBuf>,
    argument: &'static str,
    arguments: &mut impl Iterator<Item = String>,
) -> Result<(), CliError> {
    if slot.is_some() {
        return Err(CliError::DuplicateArgument(argument));
    }
    let value = arguments.next().ok_or(CliError::MissingValue(argument))?;
    *slot = Some(PathBuf::from(value));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{CliError, parse_arguments};

    #[test]
    fn parser_keeps_current_and_previous_import_graphs_separate() {
        // Arrange
        let arguments = [
            "--input",
            "next.yml",
            "--output",
            "generated.rs",
            "--capability-output",
            "capability.rs",
            "--server-output",
            "server.rs",
            "--reference-output",
            "protocol.md",
            "--import",
            "common-next.yml",
            "--previous",
            "previous.yml",
            "--previous-import",
            "common-previous.yml",
            "--check",
        ]
        .into_iter()
        .map(str::to_owned);

        // Act
        let parsed = parse_arguments(arguments).expect("arguments should parse");

        // Assert
        assert_eq!(parsed.input, PathBuf::from("next.yml"));
        assert_eq!(
            parsed.capability_output,
            Some(PathBuf::from("capability.rs"))
        );
        assert_eq!(parsed.server_output, Some(PathBuf::from("server.rs")));
        assert_eq!(parsed.reference_output, Some(PathBuf::from("protocol.md")));
        assert_eq!(parsed.imports, [PathBuf::from("common-next.yml")]);
        assert_eq!(parsed.previous, Some(PathBuf::from("previous.yml")));
        assert_eq!(
            parsed.previous_imports,
            [PathBuf::from("common-previous.yml")]
        );
        assert!(parsed.check);
    }

    #[test]
    fn previous_import_requires_a_previous_capability_source() {
        // Arrange
        let arguments = [
            "--input",
            "next.yml",
            "--output",
            "generated.rs",
            "--previous-import",
            "common-previous.yml",
        ]
        .into_iter()
        .map(str::to_owned);

        // Act
        let error = parse_arguments(arguments).expect_err("orphan previous import must fail");

        // Assert
        assert!(matches!(error, CliError::PreviousImportWithoutPrevious));
    }
}
