//! Command-line front end for Coda.

use coda::digest;
use coda::gen::{self, GenOptions, Scale, Style};
use coda::render::{render, wav_bytes};
use coda::score;
use coda::wave::Wave;
use std::path::Path;
use std::{env, fs, process::ExitCode};

const HELP: &str = "\
coda - deterministic sound compiler: score text in, WAV out

USAGE
  coda render <score.coda> [options]
  coda seed <number> [options]
  coda info <score.coda>
  coda digest <score.coda> [options]
  coda album <number> [options]
  coda help

OPTIONS
  -o, --out <file>     Output WAV path (default: out.wav)
      --sample-rate N  Samples per second (default: 44100)
      --bars N         Bars to generate for `seed` (default: 4)
      --tracks N       Tracks to write for `album` (default: 3)
      --wave <name>    sine | square | triangle | saw | noise | pulse | organ | pluck
      --scale <name>   major | minor | pentatonic | chromatic (seed only)
      --style <name>   flow | arp | chords (seed and album)
      --tempo <bpm>    Override tempo (render and seed)

SCORE DIRECTIVES
  tempo <bpm>          wave <name>          gain <0..1>
  attack/decay/sustain/release <seconds>
  note <pitch> <beats> [velocity] [pan]
  at <beat> <pitch> <beats> [velocity] [pan]
  rest <beats>          pan <-1..1>   repeat <n>
  vibrato <rate_hz> <cents>
  env <a> <d> <s> <r>   per-note ADSR; env default resets

EXIT CODES
  0 success    2 usage or IO error

No dependencies, no network. Identical inputs produce identical WAV bytes.
";

fn fail(message: &str, fix: &str) -> ExitCode {
    eprintln!("error: {message}");
    eprintln!("Fix: {fix}");
    ExitCode::from(2)
}

fn read_score(path: &str) -> Result<score::Score, ExitCode> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            return Err(fail(
                &format!("cannot read '{path}': {e}"),
                "check that the score file exists and is UTF-8",
            ))
        }
    };
    score::parse(&text).map_err(|e| fail(&e, "fix the score directive reported above"))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || matches!(args[0].as_str(), "help" | "-h" | "--help") {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    }
    let command = args[0].clone();

    let mut output = "out.wav".to_string();
    let mut sample_rate = 44_100_u32;
    let mut tracks = 3_u32;
    let mut options = GenOptions::default();
    let mut wave_override: Option<Wave> = None;
    let mut tempo_override: Option<f64> = None;
    let mut positional: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        let need = |i: usize| -> Option<&str> { args.get(i + 1).map(|s| s.as_str()) };
        match args[i].as_str() {
            "-o" | "--out" => match need(i) {
                Some(v) => {
                    output = v.to_string();
                    i += 2;
                }
                None => {
                    return fail(
                        "--out needs a file path",
                        "coda render score.coda -o out.wav",
                    )
                }
            },
            "--sample-rate" => match need(i).and_then(|v| v.parse::<u32>().ok()) {
                Some(v) if v > 0 => {
                    sample_rate = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--sample-rate must be a positive integer",
                        "coda render score.coda --sample-rate 22050",
                    )
                }
            },
            "--bars" => match need(i).and_then(|v| v.parse::<u32>().ok()) {
                Some(v) => {
                    options.bars = v;
                    i += 2;
                }
                None => {
                    return fail(
                        "--bars must be a non-negative integer",
                        "coda seed 42 --bars 4",
                    )
                }
            },
            "--tracks" => match need(i).and_then(|v| v.parse::<u32>().ok()) {
                Some(v) if v > 0 => {
                    tracks = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--tracks must be a positive integer",
                        "coda album 42 --tracks 5",
                    )
                }
            },
            "--wave" => match need(i).and_then(Wave::parse) {
                Some(w) => {
                    wave_override = Some(w);
                    i += 2;
                }
                None => {
                    return fail(
                        "unknown wave",
                        "use sine, square, triangle, saw, noise, pulse, organ or pluck",
                    )
                }
            },
            "--scale" => match need(i).and_then(Scale::parse) {
                Some(s) => {
                    options.scale = s;
                    i += 2;
                }
                None => return fail("unknown scale", "use major, minor, pentatonic or chromatic"),
            },
            "--style" => match need(i).and_then(Style::parse) {
                Some(s) => {
                    options.style = s;
                    i += 2;
                }
                None => return fail("unknown style", "use flow, arp or chords"),
            },
            "--tempo" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v > 0.0 => {
                    tempo_override = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--tempo must be a positive number",
                        "coda seed 42 --tempo 96",
                    )
                }
            },
            flag if flag.starts_with('-') => {
                return fail(&format!("unknown option '{flag}'"), "coda help")
            }
            value => {
                positional.push(value.to_string());
                i += 1;
            }
        }
    }

    if command == "album" {
        if positional.is_empty() {
            return fail(
                "album needs a seed number",
                "coda album 42 --tracks 3 -o album/",
            );
        }
        let base = match positional[0].parse::<u64>() {
            Ok(n) => n,
            Err(_) => return fail("album seed must be a non-negative integer", "coda album 42"),
        };
        if let Some(w) = wave_override {
            options.wave = w;
        }
        if let Some(t) = tempo_override {
            options.tempo = t;
        }
        if let Err(e) = fs::create_dir_all(&output) {
            return fail(
                &format!("cannot create album directory '{output}': {e}"),
                "check that the parent directory is writable",
            );
        }
        let mut fingerprints = Vec::new();
        for n in 0..tracks {
            let piece = gen::generate(base + n as u64, options);
            let samples = render(&piece, sample_rate);
            let bytes = wav_bytes(&samples, sample_rate);
            let fingerprint = digest::fingerprint(&bytes);
            fingerprints.push(fingerprint.clone());
            let filename = format!("track-{:02}.wav", n + 1);
            let path = Path::new(&output).join(&filename);
            if let Err(e) = fs::write(&path, &bytes) {
                return fail(
                    &format!("cannot write '{}': {e}", path.display()),
                    "check that the album directory is writable",
                );
            }
            println!("{filename}  {fingerprint}  {} samples", samples.len());
        }
        let album_digest = digest::fingerprint(fingerprints.join("\n").as_bytes());
        let track_list = fingerprints
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let manifest = format!(
            concat!(
                "{{\n",
                "  \"seed\": {base},\n",
                "  \"tracks\": {tracks},\n",
                "  \"bars\": {bars},\n",
                "  \"scale\": \"{scale}\",\n",
                "  \"style\": \"{style}\",\n",
                "  \"wave\": \"{wave}\",\n",
                "  \"tempo\": {tempo},\n",
                "  \"sample_rate\": {sample_rate},\n",
                "  \"album_digest\": \"{album_digest}\",\n",
                "  \"track_digests\": [{track_list}]\n",
                "}}\n"
            ),
            base = base,
            tracks = tracks,
            bars = options.bars,
            scale = options.scale.name(),
            style = options.style.name(),
            wave = options.wave.name(),
            tempo = options.tempo,
            sample_rate = sample_rate,
            album_digest = album_digest,
            track_list = track_list,
        );
        let manifest_path = Path::new(&output).join("manifest.json");
        if let Err(e) = fs::write(&manifest_path, manifest) {
            return fail(
                &format!("cannot write '{}': {e}", manifest_path.display()),
                "check that the album directory is writable",
            );
        }
        println!("album {album_digest}  {tracks} tracks");
        println!("manifest {}", manifest_path.display());
        return ExitCode::SUCCESS;
    }

    let composed = match command.as_str() {
        "render" | "info" | "digest" => {
            if positional.is_empty() {
                return fail(
                    &format!("{command} needs a score file"),
                    &format!("coda {command} <score.coda>"),
                );
            }
            let mut score = match read_score(&positional[0]) {
                Ok(s) => s,
                Err(code) => return code,
            };
            if let Some(w) = wave_override {
                score.wave = w;
            }
            if let Some(t) = tempo_override {
                score.tempo = t;
            }
            score
        }
        "seed" => {
            if positional.is_empty() {
                return fail(
                    "seed needs a number",
                    "coda seed 42 [--bars 4] [--scale minor]",
                );
            }
            let number = match positional[0].parse::<u64>() {
                Ok(n) => n,
                Err(_) => return fail("seed must be a non-negative integer", "coda seed 42"),
            };
            if let Some(w) = wave_override {
                options.wave = w;
            }
            if let Some(t) = tempo_override {
                options.tempo = t;
            }
            gen::generate(number, options)
        }
        other => return fail(&format!("unknown command '{other}'"), "coda help"),
    };

    if command == "info" {
        let sounding = composed.notes.iter().filter(|n| n.pitch != "-").count();
        let rests = composed.notes.len() - sounding;
        let beats: f64 = composed
            .notes
            .iter()
            .map(|n| n.start + n.beats)
            .fold(0.0_f64, f64::max);
        let seconds = beats * 60.0 / composed.tempo;
        println!("wave       {}", composed.wave.name());
        println!("tempo      {}", composed.tempo);
        println!("gain       {}", composed.gain);
        println!(
            "envelope   A{} D{} S{} R{}",
            composed.envelope.attack,
            composed.envelope.decay,
            composed.envelope.sustain,
            composed.envelope.release
        );
        println!(
            "vibrato    {} Hz / {} cents",
            composed.vibrato.rate, composed.vibrato.cents
        );
        println!("notes      {} sounding, {} rests", sounding, rests);
        println!("length     {beats} beats / {seconds:.3}s");
        return ExitCode::SUCCESS;
    }

    let samples = render(&composed, sample_rate);
    let bytes = wav_bytes(&samples, sample_rate);
    let fingerprint = digest::fingerprint(&bytes);

    if command == "digest" {
        println!(
            "{fingerprint}  {} bytes  {} samples",
            bytes.len(),
            samples.len()
        );
        return ExitCode::SUCCESS;
    }

    match fs::write(&output, &bytes) {
        Ok(()) => {
            println!(
                "wrote {} ({} bytes, {} samples, {} notes, digest {fingerprint})",
                output,
                bytes.len(),
                samples.len(),
                composed.notes.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => fail(
            &format!("cannot write '{output}': {e}"),
            "check that the output directory exists and is writable",
        ),
    }
}
