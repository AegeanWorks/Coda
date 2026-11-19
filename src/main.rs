//! Command-line front end for Coda.

use coda::compare;
use coda::digest;
use coda::fade;
use coda::gen::{self, GenOptions, Scale, Style};
use coda::join;
use coda::lint;
use coda::midi_import;
use coda::normalize;
use coda::render::{render, wav_bytes, Frame};
use coda::score;
use coda::stats;
use coda::templates;
use coda::transpose;
use coda::vary;
use coda::wave::Wave;
use std::path::Path;
use std::{env, fs, process::ExitCode};

const HELP: &str = "\
coda - deterministic sound compiler: score text in, WAV out

USAGE
  coda render <score.coda> [options]
  coda seed <number> [options]
  coda info <score.coda>
  coda stats <score.coda>
  coda templates
  coda template <name> [-o out.coda]
  coda lint <score.coda>
  coda export-score <score.coda> [options]
  coda vary <score.coda> --seed N [--amount 0..1] [-o out.coda]
  coda compare <a.coda> <b.coda>
  coda join <a.coda> <b.coda> [more...] [-o out.wav]
  coda normalize <in.wav> [-o out.wav] [--peak 0..1]
  coda import-midi <file.mid> [-o out.coda]
  coda digest <score.coda> [options]
  coda album <number> [options]
  coda help

OPTIONS
  -o, --out <file>     Output path (default: out.wav); .mid/.midi MIDI, .coda source
      --sample-rate N  Samples per second (default: 44100)
      --fade-in S      Linear fade-in in seconds (default: 0)
      --fade-out S     Linear fade-out in seconds (default: 0)
      --bars N         Bars to generate for `seed` (default: 4)
      --tracks N       Tracks to write for `album` (default: 3)
      --wave <name>    sine | square | triangle | saw | noise | pulse | organ | pluck
      --scale <name>   major | minor | pentatonic | chromatic (seed only)
      --style <name>   flow | arp | chords (seed and album)
      --tempo <bpm>    Override tempo (render and seed)
      --semitones N    Transpose every pitched note by N semitones
      --seed N         Variation seed for `vary`
      --amount 0..1    Variation strength (default: 0.5)
      --peak 0..1      Target peak for normalize (default: 0.95)

SCORE DIRECTIVES
  tempo <bpm>          wave <name>          gain <0..1>
  attack/decay/sustain/release <seconds>
  note <pitch> <beats> [velocity] [pan]
  at <beat> <pitch> <beats> [velocity] [pan]
  rest <beats>          pan <-1..1>   repeat <n>
  inst <wave>           per-note oscillator; inst default resets
  vibrato <rate_hz> <cents>  vib <rate_hz> <cents>
  env <a> <d> <s> <r>   per-note ADSR; env default resets
  preset <name>         pluck|stab|pad|organ|bow; preset default resets

EXIT CODES
  0 success    2 usage or IO error

No dependencies, no network. Identical inputs produce identical WAV or MIDI bytes.
";

fn fail(message: &str, fix: &str) -> ExitCode {
    eprintln!("error: {message}");
    eprintln!("Fix: {fix}");
    ExitCode::from(2)
}

fn rendered_wav(
    composed: &score::Score,
    sample_rate: u32,
    fade_in: f64,
    fade_out: f64,
) -> (Vec<Frame>, Vec<u8>) {
    let mut samples = render(composed, sample_rate);
    fade::apply_fades(&mut samples, sample_rate, fade_in, fade_out);
    let bytes = wav_bytes(&samples, sample_rate);
    (samples, bytes)
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
    let mut fade_in = 0.0_f64;
    let mut fade_out = 0.0_f64;
    let mut tracks = 3_u32;
    let mut options = GenOptions::default();
    let mut wave_override: Option<Wave> = None;
    let mut tempo_override: Option<f64> = None;
    let mut semitones = 0_i32;
    let mut variation_seed = 0_u64;
    let mut variation_amount = 0.5_f64;
    let mut peak = 0.95_f64;
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
            "--fade-in" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 && v.is_finite() => {
                    fade_in = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--fade-in must be a non-negative number",
                        "coda render score.coda --fade-in 0.05",
                    )
                }
            },
            "--fade-out" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 && v.is_finite() => {
                    fade_out = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--fade-out must be a non-negative number",
                        "coda render score.coda --fade-out 0.1",
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
            "--semitones" => match need(i).and_then(|v| v.parse::<i32>().ok()) {
                Some(n) => {
                    semitones = n;
                    i += 2;
                }
                None => {
                    return fail(
                        "--semitones must be an integer",
                        "coda render score.coda --semitones 2",
                    )
                }
            },
            "--seed" => match need(i).and_then(|v| v.parse::<u64>().ok()) {
                Some(n) => {
                    variation_seed = n;
                    i += 2;
                }
                None => {
                    return fail(
                        "--seed must be a non-negative integer",
                        "coda vary score.coda --seed 42",
                    )
                }
            },
            "--amount" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    variation_amount = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--amount must be between 0 and 1",
                        "coda vary score.coda --seed 42 --amount 0.7",
                    )
                }
            },
            "--peak" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v > 0.0 && v <= 1.0 => {
                    peak = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--peak must be between 0 and 1",
                        "coda normalize in.wav --peak 0.9",
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

    if command == "templates" {
        for name in templates::names() {
            println!("{name}");
        }
        return ExitCode::SUCCESS;
    }

    if command == "template" {
        if positional.is_empty() {
            return fail("template needs a name", "coda templates lists the choices");
        }
        let source = match templates::source(&positional[0]) {
            Some(text) => text,
            None => {
                return fail(
                    &format!("unknown template '{}'", positional[0]),
                    "coda templates lists the choices",
                )
            }
        };
        let mut composed = match score::parse(source) {
            Ok(s) => s,
            Err(e) => return fail(&e, "a built-in template is invalid"),
        };
        if let Some(w) = wave_override {
            composed.wave = w;
            for note in &mut composed.notes {
                note.wave = w;
            }
        }
        if let Some(t) = tempo_override {
            composed.tempo = t;
        }
        if semitones != 0 {
            composed = transpose::transpose(&composed, semitones);
        }
        let path = if output == "out.wav" {
            format!("{}.coda", positional[0])
        } else {
            output.clone()
        };
        let text = score::serialize(&composed);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, template {})",
                    text.len(),
                    composed.notes.len(),
                    positional[0]
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "normalize" {
        if positional.is_empty() {
            return fail(
                "normalize needs an input WAV",
                "coda normalize in.wav [-o out.wav]",
            );
        }
        let bytes = match fs::read(&positional[0]) {
            Ok(b) => b,
            Err(e) => {
                return fail(
                    &format!("cannot read '{}': {e}", positional[0]),
                    "check that the WAV file exists and is readable",
                )
            }
        };
        let scaled = match normalize::normalize(&bytes, peak) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "normalized.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &scaled) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, peak {peak})", scaled.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "join" {
        if positional.len() < 2 {
            return fail(
                "join needs at least two score files",
                "coda join a.coda b.coda [-o out.wav]",
            );
        }
        let mut parts = Vec::new();
        for path in &positional {
            let mut score = match read_score(path) {
                Ok(s) => s,
                Err(code) => return code,
            };
            if let Some(w) = wave_override {
                score.wave = w;
                for note in &mut score.notes {
                    note.wave = w;
                }
            }
            if let Some(t) = tempo_override {
                score.tempo = t;
            }
            let score = if semitones == 0 {
                score
            } else {
                transpose::transpose(&score, semitones)
            };
            let (_, bytes) = rendered_wav(&score, sample_rate, fade_in, fade_out);
            parts.push(bytes);
        }
        let joined = match join::join_wavs(&parts) {
            Ok(bytes) => bytes,
            Err(e) => return fail(&e, "all sections must be rendered at the same sample rate"),
        };
        return match fs::write(&output, &joined) {
            Ok(()) => {
                println!(
                    "wrote {output} ({} bytes, {} sections, {} samples)",
                    joined.len(),
                    parts.len(),
                    (joined.len() - 44) / 4
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{output}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "import-midi" {
        if positional.is_empty() {
            return fail(
                "import-midi needs a MIDI file",
                "coda import-midi song.mid [-o song.coda]",
            );
        }
        let bytes = match fs::read(&positional[0]) {
            Ok(b) => b,
            Err(e) => {
                return fail(
                    &format!("cannot read '{}': {e}", positional[0]),
                    "check that the MIDI file exists and is readable",
                )
            }
        };
        let imported = match midi_import::import_midi(&bytes) {
            Ok(s) => s,
            Err(e) => return fail(&e, "only format-0 or format-1 MIDI files are supported"),
        };
        let path = if output == "out.wav" {
            "imported.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&imported);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, tempo {})",
                    text.len(),
                    imported.notes.len(),
                    imported.tempo
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "compare" {
        if positional.len() < 2 {
            return fail(
                "compare needs two score files",
                "coda compare a.coda b.coda",
            );
        }
        let left = match read_score(&positional[0]) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let right = match read_score(&positional[1]) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let result = compare::compare(&left, &right, sample_rate);
        println!("identical: {}", if result.identical { "yes" } else { "no" });
        for difference in &result.differences {
            println!("difference: {difference}");
        }
        println!(
            "audio: {}",
            if result.audio_identical {
                "identical"
            } else {
                "different"
            }
        );
        return ExitCode::SUCCESS;
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
            let (samples, bytes) = rendered_wav(&piece, sample_rate, fade_in, fade_out);
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
        "render" | "info" | "stats" | "lint" | "export-score" | "vary" | "digest" => {
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
                // Notes carry their own oscillator, so the override must reach
                // them as well as the score default.
                for note in &mut score.notes {
                    note.wave = w;
                }
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

    let composed = if semitones == 0 {
        composed
    } else {
        transpose::transpose(&composed, semitones)
    };

    if command == "vary" {
        let varied = vary::vary(&composed, variation_seed, variation_amount);
        let path = if output == "out.wav" {
            "variation.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&varied);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, seed {variation_seed}, amount {variation_amount})",
                    text.len(),
                    varied.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "stats" {
        print!("{}", stats::report(&composed));
        return ExitCode::SUCCESS;
    }

    if command == "lint" {
        let warnings = lint::lint(&composed);
        for warning in &warnings {
            println!("WARN {}", warning.message);
        }
        if warnings.is_empty() {
            println!("OK: no warnings");
        }
        return ExitCode::SUCCESS;
    }

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

    if command == "export-score" {
        let path = if output == "out.wav" {
            "out.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&composed);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, score)",
                    text.len(),
                    composed.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "digest" {
        let (samples, bytes) = rendered_wav(&composed, sample_rate, fade_in, fade_out);
        let fingerprint = digest::fingerprint(&bytes);
        println!(
            "{fingerprint}  {} bytes  {} samples",
            bytes.len(),
            samples.len()
        );
        return ExitCode::SUCCESS;
    }

    let lowered = output.to_ascii_lowercase();
    let is_midi = lowered.ends_with(".mid") || lowered.ends_with(".midi");
    let is_score = lowered.ends_with(".coda");
    let (bytes, summary) = if is_score {
        let text = score::serialize(&composed);
        let summary = format!(
            "{} bytes, {} notes, score",
            text.len(),
            composed.notes.len()
        );
        (text.into_bytes(), summary)
    } else if is_midi {
        let bytes = coda::midi::midi_bytes(&composed);
        let summary = format!(
            "{} bytes, {} notes, MIDI",
            bytes.len(),
            composed.notes.len()
        );
        (bytes, summary)
    } else {
        let (samples, bytes) = rendered_wav(&composed, sample_rate, fade_in, fade_out);
        let fingerprint = digest::fingerprint(&bytes);
        let summary = format!(
            "{} bytes, {} samples, {} notes, digest {fingerprint}",
            bytes.len(),
            samples.len(),
            composed.notes.len()
        );
        (bytes, summary)
    };

    match fs::write(&output, &bytes) {
        Ok(()) => {
            println!("wrote {output} ({summary})");
            ExitCode::SUCCESS
        }
        Err(e) => fail(
            &format!("cannot write '{output}': {e}"),
            "check that the output directory exists and is writable",
        ),
    }
}
