use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use clap::Parser;
use rayon::prelude::*;

use imgsmith::{Error, Fit, Flip, Format, Gravity, Options, Outcome, parse_crop, process};

/// Image conversion, resizing, cropping and thumbnailing as one static binary.
///
/// It does the eight things scripts actually do and refuses the rest: no PDF,
/// no SVG, no animation, no filters, no plugins, no delegates.
#[derive(Parser, Debug)]
#[command(
    name = "imgsmith",
    version,
    about = "Convert, resize, crop and thumbnail images. One binary, no delegates.",
    after_help = "Examples:
  imgsmith photo.jpg -o photo.webp
  imgsmith photo.jpg -o small.jpg --width 1920
  imgsmith photo.jpg -o thumb.jpg --thumb 400
  imgsmith photos/ -o thumbs/ --width 800 --quality 80
  imgsmith shots/ -o web/ --format webp --auto-orient --strip

Exit codes: 0 all files converted, 1 one or more files failed, 2 bad usage."
)]
struct Cli {
    /// Input files or directories (directories are walked recursively).
    #[arg(required = true, value_name = "INPUT")]
    inputs: Vec<PathBuf>,

    /// Output file, or output directory for batches.
    #[arg(short, long, value_name = "PATH", required = true)]
    out: PathBuf,

    /// Output format, overriding the extension.
    #[arg(short, long, value_enum)]
    format: Option<Format>,

    /// Target width in pixels.
    #[arg(short, long)]
    width: Option<u32>,

    /// Target height in pixels.
    #[arg(long)]
    height: Option<u32>,

    /// How the target size is applied.
    #[arg(long, value_enum, default_value_t = Fit::Inside)]
    fit: Fit,

    /// Crop to WxH or WxH+X+Y before scaling.
    #[arg(long, value_name = "WxH[+X+Y]")]
    crop: Option<String>,

    /// Where an unpositioned crop sits.
    #[arg(long, value_enum, default_value_t = Gravity::Centre)]
    gravity: Gravity,

    /// Rotate clockwise by 90, 180 or 270 degrees.
    #[arg(long, value_parser = clap::value_parser!(u16))]
    rotate: Option<u16>,

    /// Mirror horizontally (h) or vertically (v).
    #[arg(long, value_enum)]
    flip: Option<Flip>,

    /// Use the EXIF orientation tag, then drop it from the output.
    #[arg(long)]
    auto_orient: bool,

    /// Write no metadata at all.
    #[arg(long)]
    strip: bool,

    /// Quality 1-100 for JPEG and AVIF. WebP output is lossless.
    #[arg(long)]
    quality: Option<u8>,

    /// Shorthand thumbnail: fit inside NxN, never upscale.
    #[arg(long, value_name = "N")]
    thumb: Option<u32>,

    /// Never scale an image up.
    #[arg(long)]
    no_upscale: bool,

    /// Files to process at once.
    #[arg(short, long, value_name = "N")]
    jobs: Option<usize>,

    /// Overwrite existing outputs.
    #[arg(long)]
    force: bool,

    /// Print what would be written, write nothing.
    #[arg(long)]
    dry_run: bool,

    /// Refuse images above N megapixels (0 disables the guard).
    #[arg(long, value_name = "MP", default_value_t = 100)]
    max_mp: u64,

    /// Print nothing except errors.
    #[arg(short, long)]
    quiet: bool,

    /// Print one line per file.
    #[arg(short, long)]
    verbose: bool,
}

struct Job {
    input: PathBuf,
    output: PathBuf,
    format: Format,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("imgsmith: {err}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, Error> {
    // --- validation, before any file is touched -------------------------
    for (flag, value) in [
        ("--width", cli.width),
        ("--height", cli.height),
        ("--thumb", cli.thumb),
    ] {
        if value == Some(0) {
            return Err(Error::Usage(format!("{flag} must be greater than zero")));
        }
    }
    if cli.jobs == Some(0) {
        return Err(Error::Usage("--jobs must be greater than zero".into()));
    }
    if let Some(q) = cli.quality
        && !(1..=100).contains(&q)
    {
        return Err(Error::Usage(format!("--quality accepts 1 to 100, got {q}")));
    }
    if cli.thumb.is_some() && (cli.width.is_some() || cli.height.is_some()) {
        return Err(Error::Usage(
            "--thumb already sets the size, do not combine it with --width or --height".into(),
        ));
    }
    let crop = match &cli.crop {
        Some(raw) => Some(parse_crop(raw).map_err(Error::Usage)?),
        None => None,
    };
    if let Some(deg) = cli.rotate
        && !matches!(deg, 90 | 180 | 270)
    {
        return Err(Error::Usage("--rotate accepts 90, 180 or 270".into()));
    }

    let thumbnail = cli.thumb.is_some();
    let (width, height) = match cli.thumb {
        Some(n) => (Some(n), Some(n)),
        None => (cli.width, cli.height),
    };
    let opts = Options {
        width,
        height,
        fit: cli.fit,
        crop,
        gravity: cli.gravity,
        rotate: cli.rotate,
        flip: cli.flip,
        auto_orient: cli.auto_orient,
        strip: cli.strip,
        quality: cli.quality,
        thumbnail,
        allow_upscale: !(cli.no_upscale || thumbnail),
        max_pixels: cli.max_mp.saturating_mul(1_000_000),
    };

    // --- expand inputs ---------------------------------------------------
    let mut files: Vec<(PathBuf, Option<PathBuf>)> = Vec::new();
    let mut saw_dir = false;
    for input in &cli.inputs {
        if input.is_dir() {
            saw_dir = true;
            let mut found = Vec::new();
            walk(input, &mut found).map_err(|e| Error::io(input, e))?;
            if found.is_empty() {
                return Err(Error::Usage(format!(
                    "{}: no images found (looked for the formats this tool reads)",
                    input.display()
                )));
            }
            found.sort();
            for path in found {
                let relative = path.strip_prefix(input).ok().map(|p| p.to_path_buf());
                files.push((path, relative));
            }
        } else if input.is_file() {
            files.push((input.clone(), None));
        } else {
            return Err(Error::Usage(format!(
                "{}: no such file or directory",
                input.display()
            )));
        }
    }

    let out_is_dir = saw_dir
        || files.len() > 1
        || cli.out.is_dir()
        || cli.out.to_string_lossy().ends_with('/')
        || (cli.format.is_some() && cli.out.extension().is_none());

    if !out_is_dir && files.len() > 1 {
        return Err(Error::Usage(
            "--out must be a directory when converting more than one file".into(),
        ));
    }

    // --- plan jobs -------------------------------------------------------
    let mut jobs = Vec::with_capacity(files.len());
    for (input, relative) in files {
        let format = resolve_format(&cli, &input, out_is_dir)?;
        let output = if out_is_dir {
            let mut target = match &relative {
                Some(rel) => cli.out.join(rel),
                None => cli.out.join(
                    input
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("out")),
                ),
            };
            target.set_extension(format.extension());
            target
        } else {
            cli.out.clone()
        };
        // Quality only means something for lossy formats. Refusing beats
        // accepting a number that changes nothing.
        if let Some(q) = cli.quality {
            if format == Format::Webp {
                return Err(Error::Usage(format!(
                    "--quality {q} cannot apply to WebP output: this tool writes lossless WebP only. \
                     Drop --quality, or choose jpeg or avif"
                )));
            }
            if !matches!(format, Format::Jpeg | Format::Avif) {
                return Err(Error::Usage(format!(
                    "--quality {q} has no effect on {} output; it applies to jpeg and avif",
                    format.extension()
                )));
            }
        }
        jobs.push(Job {
            input,
            output,
            format,
        });
    }

    if cli.dry_run {
        for job in &jobs {
            eprintln!(
                "would write {} (from {})",
                job.output.display(),
                job.input.display()
            );
        }
        eprintln!("{} file(s) planned, nothing written", jobs.len());
        return Ok(ExitCode::SUCCESS);
    }

    // --- run -------------------------------------------------------------
    let started = Instant::now();
    let threads = cli
        .jobs
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        })
        .max(1);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|e| Error::Usage(format!("could not start {threads} workers: {e}")))?;

    let results: Vec<Result<Outcome, Error>> = pool.install(|| {
        jobs.par_iter()
            .map(|job| process(&job.input, &job.output, job.format, &opts, cli.force))
            .collect()
    });

    // --- report ----------------------------------------------------------
    let mut failures = 0usize;
    let mut total_in = 0u64;
    let mut total_out = 0u64;
    for (job, result) in jobs.iter().zip(results.iter()) {
        match result {
            Ok(outcome) => {
                total_in += outcome.in_bytes;
                total_out += outcome.out_bytes;
                if cli.verbose && !cli.quiet {
                    eprintln!(
                        "{} → {}  {}x{} → {}x{}  {} → {}",
                        job.input.display(),
                        job.output.display(),
                        outcome.in_dims.0,
                        outcome.in_dims.1,
                        outcome.out_dims.0,
                        outcome.out_dims.1,
                        human_bytes(outcome.in_bytes),
                        human_bytes(outcome.out_bytes)
                    );
                }
            }
            Err(err) => {
                failures += 1;
                eprintln!("imgsmith: {err}");
            }
        }
    }

    if !cli.quiet {
        let elapsed = started.elapsed();
        if jobs.len() == 1 && failures == 0 {
            if let Ok(outcome) = &results[0] {
                eprintln!(
                    "{}x{} → {}x{}, {} → {}, {:.2}s",
                    outcome.in_dims.0,
                    outcome.in_dims.1,
                    outcome.out_dims.0,
                    outcome.out_dims.1,
                    human_bytes(outcome.in_bytes),
                    human_bytes(outcome.out_bytes),
                    elapsed.as_secs_f64()
                );
            }
        } else {
            eprintln!(
                "{} of {} files, {} → {}, {:.2}s",
                jobs.len() - failures,
                jobs.len(),
                human_bytes(total_in),
                human_bytes(total_out),
                elapsed.as_secs_f64()
            );
        }
    }

    Ok(if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn resolve_format(cli: &Cli, input: &Path, out_is_dir: bool) -> Result<Format, Error> {
    if let Some(format) = cli.format {
        return Ok(format);
    }
    if !out_is_dir {
        if let Some(format) = cli
            .out
            .extension()
            .and_then(|e| e.to_str())
            .and_then(Format::from_extension)
        {
            return Ok(format);
        }
        return Err(Error::Usage(format!(
            "{}: unknown output extension, name the file .png/.jpg/.webp or pass --format",
            cli.out.display()
        )));
    }
    if let Some(format) = input
        .extension()
        .and_then(|e| e.to_str())
        .and_then(Format::from_extension)
    {
        return Ok(format);
    }
    Err(Error::Usage(format!(
        "{}: unknown input extension, pass --format to choose the output format",
        input.display()
    )))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(Format::from_extension)
            .is_some()
        {
            out.push(path);
        }
    }
    Ok(())
}

fn human_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else if b >= KB {
        format!("{:.0} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}
