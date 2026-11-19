//! Command-line front end for Coda.

use coda::analyze;
use coda::autopan;
use coda::compare;
use coda::concat;
use coda::diatonic;
use coda::digest;
use coda::echo;
use coda::fade;
use coda::fade_presets;
use coda::gen::{self, GenOptions, Scale, Style};
use coda::humanize;
use coda::join;
use coda::lint;
use coda::loop_score;
use coda::midi_import;
use coda::mix;
use coda::normalize;
use coda::quantize;
use coda::render::{render, wav_bytes, Frame};
use coda::resample;
use coda::reverse;
use coda::score;
use coda::stats;
use coda::stretch;
use coda::swing;
use coda::templates;
use coda::transpose;
use coda::trim;
use coda::vary;
use coda::wav_reverse;
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
  coda reverse <score.coda> [-o out.coda]
  coda quantize <score.coda> [--grid 0..1 beats] [-o out.coda]
  coda humanize <score.coda> --seed N [--amount 0..1] [-o out.coda]
  coda diatonic <score.coda> --steps N [--scale name] [-o out.coda]
  coda swing <score.coda> [--ratio 0..1] [-o out.coda]
  coda stretch <score.coda> --factor F [-o out.coda]
  coda analyze <in.wav>
  coda trim <in.wav> [--start S] [--end S] [--trim-silence] [-o out.wav]
  coda fade <in.wav> --preset NAME [-o out.wav]
  coda loop <score.coda> --times N [-o out.coda]
  coda compare <a.coda> <b.coda>
  coda concat <a.coda> <b.coda> [...] [-o out.coda]
  coda resample <in.wav> --rate HZ [-o out.wav]
  coda join <a.coda> <b.coda> [more...] [-o out.wav] [--crossfade S]
  coda normalize <in.wav> [-o out.wav] [--peak 0..1 | --rms-target dBFS]
  coda reverse-wav <in.wav> [-o out.wav]
  coda mix <a.wav> <b.wav> [-o out.wav] [--gain-b 0..1]
  coda import-midi <file.mid> [-o out.coda]
  coda digest <score.coda> [options]
  coda album <number> [options]
  coda help

OPTIONS
  -o, --out <file>     Output path (default: out.wav); .mid/.midi MIDI, .coda source
      --sample-rate N  Samples per second (default: 44100)
      --fade-in S      Linear fade-in in seconds (default: 0)
      --fade-out S     Linear fade-out in seconds (default: 0)
      --echo-delay S   Feedback delay in seconds (default: 0)
      --echo-feedback  Echo gain 0..1 (default: 0.35)
      --autopan-rate   Pan LFO rate in Hz (default: 0)
      --autopan-depth  Pan depth 0..1 (default: 0)
      --grid B         Quantization grid in beats (default: 0.25)
      --gain-b 0..1    Gain on the second WAV in mix (default: 0.5)
      --steps N        Scale degrees to move in diatonic (default: 0)
      --ratio 0..1     Swing amount (default: 0.6)
      --factor F       Time-stretch factor (default: 1)
      --crossfade S    Overlap between joined sections in seconds (default: 0)
      --start S / --end S  Time range for trim
      --trim-silence   Auto-trim silence below --threshold
      --threshold DB   Silence threshold for trim (default: -60 dBFS)
      --preset NAME    Fade preset: short|gentle|long|tail|attack
      --times N        Repetitions for loop (default: 1)
      --rate HZ        Target sample rate for resample
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
      --rms-target DB  Target RMS in dBFS (-100..0) instead of peak

SCORE DIRECTIVES
  tempo <bpm>          wave <name>          gain <0..1>
  attack/decay/sustain/release <seconds>
  note <pitch> <beats> [velocity] [pan]
  at <beat> <pitch> <beats> [velocity] [pan]  (`-` places a rest)
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

struct AudioOptions {
    sample_rate: u32,
    fade_in: f64,
    fade_out: f64,
    echo_delay: f64,
    echo_feedback: f64,
    pan_rate: f64,
    pan_depth: f64,
}

fn rendered_wav(composed: &score::Score, options: &AudioOptions) -> (Vec<Frame>, Vec<u8>) {
    let rate = options.sample_rate;
    let mut samples = render(composed, rate);
    echo::apply_echo(
        &mut samples,
        rate,
        options.echo_delay,
        options.echo_feedback,
    );
    autopan::apply_autopan(&mut samples, rate, options.pan_rate, options.pan_depth);
    fade::apply_fades(&mut samples, rate, options.fade_in, options.fade_out);
    let bytes = wav_bytes(&samples, rate);
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
    let mut echo_delay = 0.0_f64;
    let mut echo_feedback = 0.35_f64;
    let mut pan_rate = 0.0_f64;
    let mut pan_depth = 0.0_f64;
    let mut grid = 0.25_f64;
    let mut gain_b = 0.5_f64;
    let mut steps = 0_i32;
    let mut ratio = 0.6_f64;
    let mut factor = 1.0_f64;
    let mut rms_target: Option<f64> = None;
    let mut crossfade = 0.0_f64;
    let mut trim_start = 0.0_f64;
    let mut trim_end: Option<f64> = None;
    let mut trim_silence = false;
    let mut trim_threshold = -60.0_f64;
    let mut fade_preset: Option<String> = None;
    let mut times = 1_usize;
    let mut target_rate: Option<u32> = None;
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
            "--echo-delay" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 && v.is_finite() => {
                    echo_delay = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--echo-delay must be a non-negative number",
                        "coda render score.coda --echo-delay 0.12",
                    )
                }
            },
            "--echo-feedback" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    echo_feedback = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--echo-feedback must be between 0 and 1",
                        "coda render score.coda --echo-feedback 0.4",
                    )
                }
            },
            "--autopan-rate" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 && v.is_finite() => {
                    pan_rate = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--autopan-rate must be a non-negative number",
                        "coda render score.coda --autopan-rate 0.2",
                    )
                }
            },
            "--autopan-depth" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    pan_depth = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--autopan-depth must be between 0 and 1",
                        "coda render score.coda --autopan-depth 0.6",
                    )
                }
            },
            "--grid" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v > 0.0 && v.is_finite() => {
                    grid = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--grid must be a positive finite number",
                        "coda quantize score.coda --grid 0.25",
                    )
                }
            },
            "--gain-b" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    gain_b = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--gain-b must be between 0 and 1",
                        "coda mix a.wav b.wav --gain-b 0.5",
                    )
                }
            },
            "--steps" => match need(i).and_then(|v| v.parse::<i32>().ok()) {
                Some(n) => {
                    steps = n;
                    i += 2;
                }
                None => {
                    return fail(
                        "--steps must be an integer",
                        "coda diatonic score.coda --steps 2",
                    )
                }
            },
            "--ratio" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    ratio = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--ratio must be between 0 and 1",
                        "coda swing score.coda --ratio 0.6",
                    )
                }
            },
            "--factor" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v > 0.0 && v.is_finite() => {
                    factor = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--factor must be a positive finite number",
                        "coda stretch score.coda --factor 2",
                    )
                }
            },
            "--crossfade" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 && v.is_finite() => {
                    crossfade = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--crossfade must be a non-negative number",
                        "coda join a.coda b.coda --crossfade 0.05",
                    )
                }
            },
            "--start" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 && v.is_finite() => {
                    trim_start = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--start must be a non-negative number",
                        "coda trim in.wav --start 1.0",
                    )
                }
            },
            "--end" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 && v.is_finite() => {
                    trim_end = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--end must be a non-negative number",
                        "coda trim in.wav --end 2.0",
                    )
                }
            },
            "--trim-silence" => {
                trim_silence = true;
                i += 1;
            }
            "--threshold" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (-100.0..=0.0).contains(&v) => {
                    trim_threshold = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--threshold must be between -100 and 0 dBFS",
                        "coda trim in.wav --trim-silence --threshold -60",
                    )
                }
            },
            "--preset" => match need(i) {
                Some(name) => {
                    fade_preset = Some(name.to_string());
                    i += 2;
                }
                None => return fail("--preset needs a name", "coda fade in.wav --preset short"),
            },
            "--times" => match need(i).and_then(|v| v.parse::<usize>().ok()) {
                Some(n) if n > 0 => {
                    times = n;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--times must be a positive integer",
                        "coda loop score.coda --times 3",
                    )
                }
            },
            "--rate" => match need(i).and_then(|v| v.parse::<u32>().ok()) {
                Some(v) if v > 0 && v <= 384_000 => {
                    target_rate = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--rate must be between 1 and 384000 Hz",
                        "coda resample in.wav --rate 22050",
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
            "--rms-target" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (-100.0..=0.0).contains(&v) => {
                    rms_target = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--rms-target must be between -100 and 0 dBFS",
                        "coda normalize in.wav --rms-target -14",
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

    let audio = AudioOptions {
        sample_rate,
        fade_in,
        fade_out,
        echo_delay,
        echo_feedback,
        pan_rate,
        pan_depth,
    };

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

    if command == "resample" {
        if positional.is_empty() {
            return fail(
                "resample needs an input WAV",
                "coda resample in.wav --rate 22050",
            );
        }
        let rate = match target_rate {
            Some(r) => r,
            None => return fail("resample needs --rate", "coda resample in.wav --rate 22050"),
        };
        let bytes = match fs::read(&positional[0]) {
            Ok(b) => b,
            Err(e) => {
                return fail(
                    &format!("cannot read '{}': {e}", positional[0]),
                    "check that the WAV file exists and is readable",
                )
            }
        };
        let converted = match resample::resample(&bytes, rate) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "resampled.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &converted) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, {rate} Hz)", converted.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "fade" {
        if positional.is_empty() {
            return fail("fade needs an input WAV", "coda fade in.wav --preset short");
        }
        let preset = match &fade_preset {
            Some(name) => name.clone(),
            None => return fail("fade needs --preset", "coda fade in.wav --preset short"),
        };
        let bytes = match fs::read(&positional[0]) {
            Ok(b) => b,
            Err(e) => {
                return fail(
                    &format!("cannot read '{}': {e}", positional[0]),
                    "check that the WAV file exists and is readable",
                )
            }
        };
        let faded = match fade_presets::apply(&bytes, &preset) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "faded.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &faded) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, preset {preset})", faded.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "trim" {
        if positional.is_empty() {
            return fail(
                "trim needs an input WAV",
                "coda trim in.wav [--start S] [--end S]",
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
        let threshold = if trim_silence {
            Some(trim_threshold)
        } else {
            None
        };
        let cut = match trim::trim_wav(&bytes, trim_start, trim_end, threshold) {
            Ok(b) => b,
            Err(e) => return fail(&e, "check the trim range and threshold"),
        };
        let path = if output == "out.wav" {
            "trimmed.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &cut) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, trimmed)", cut.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "analyze" {
        if positional.is_empty() {
            return fail("analyze needs an input WAV", "coda analyze in.wav");
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
        return match analyze::analyze(&bytes) {
            Ok(report) => {
                print!("{report}");
                ExitCode::SUCCESS
            }
            Err(e) => fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
    }

    if command == "mix" {
        if positional.len() < 2 {
            return fail(
                "mix needs two WAV files",
                "coda mix a.wav b.wav [-o out.wav]",
            );
        }
        let mut inputs = Vec::new();
        for path in positional.iter().take(2) {
            match fs::read(path) {
                Ok(bytes) => inputs.push(bytes),
                Err(e) => {
                    return fail(
                        &format!("cannot read '{path}': {e}"),
                        "check that both WAV files exist and are readable",
                    )
                }
            }
        }
        let merged = match mix::mix_wavs(&inputs[0], &inputs[1], gain_b) {
            Ok(bytes) => bytes,
            Err(e) => return fail(&e, "both inputs must be Coda WAVs at the same sample rate"),
        };
        return match fs::write(&output, &merged) {
            Ok(()) => {
                println!(
                    "wrote {output} ({} bytes, {} frames, gain {gain_b})",
                    merged.len(),
                    (merged.len() - 44) / 4
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{output}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "reverse-wav" {
        if positional.is_empty() {
            return fail(
                "reverse-wav needs an input WAV",
                "coda reverse-wav in.wav [-o out.wav]",
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
        let reversed = match wav_reverse::reverse_wav(&bytes) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "reversed.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &reversed) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, reversed)", reversed.len());
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
        let (scaled, mode) = match rms_target {
            Some(target) => (
                match normalize::normalize_rms(&bytes, target) {
                    Ok(b) => b,
                    Err(e) => return fail(&e, "use an RMS target between -100 and 0 dBFS"),
                },
                format!("RMS {target} dBFS"),
            ),
            None => (
                match normalize::normalize(&bytes, peak) {
                    Ok(b) => b,
                    Err(e) => return fail(&e, "use a peak between 0 and 1"),
                },
                format!("peak {peak}"),
            ),
        };
        let path = if output == "out.wav" {
            "normalized.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &scaled) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, {mode})", scaled.len());
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
            let (_, bytes) = rendered_wav(&score, &audio);
            parts.push(bytes);
        }
        let overlap = (crossfade * sample_rate as f64).round() as usize;
        let joined = match join::join_wavs_crossfade(&parts, overlap) {
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

    if command == "concat" {
        if positional.len() < 2 {
            return fail(
                "concat needs at least two score files",
                "coda concat a.coda b.coda [-o out.coda]",
            );
        }
        let mut scores = Vec::new();
        for path in &positional {
            match read_score(path) {
                Ok(s) => scores.push(s),
                Err(code) => return code,
            }
        }
        let merged = match concat::concat_scores(&scores) {
            Ok(s) => s,
            Err(e) => return fail(&e, "provide at least two valid scores"),
        };
        let path = if output == "out.wav" {
            "concatenated.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&merged);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, {} scores)",
                    text.len(),
                    merged.notes.len(),
                    scores.len()
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
            let (samples, bytes) = rendered_wav(&piece, &audio);
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
        "render" | "info" | "stats" | "lint" | "export-score" | "vary" | "humanize"
        | "diatonic" | "swing" | "stretch" | "loop" | "reverse" | "quantize" | "digest" => {
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

    if command == "loop" {
        let looped = match loop_score::repeat_score(&composed, times) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use a positive repetition count"),
        };
        let path = if output == "out.wav" {
            "looped.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&looped);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, {times} repetitions)",
                    text.len(),
                    looped.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "stretch" {
        let scaled = match stretch::stretch(&composed, factor) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use a positive finite factor"),
        };
        let path = if output == "out.wav" {
            "stretched.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&scaled);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, factor {factor})",
                    text.len(),
                    scaled.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "swing" {
        let swung = match swing::swing(&composed, ratio) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use a ratio between 0 and 1"),
        };
        let path = if output == "out.wav" {
            "swung.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&swung);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, swing {ratio})",
                    text.len(),
                    swung.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "diatonic" {
        let shifted = diatonic::diatonic(&composed, steps, options.scale);
        let path = if output == "out.wav" {
            "diatonic.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&shifted);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, {steps} steps, {})",
                    text.len(),
                    shifted.notes.len(),
                    options.scale.name()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "humanize" {
        let adjusted = match humanize::humanize(&composed, variation_seed, variation_amount) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use an amount between 0 and 1"),
        };
        let path = if output == "out.wav" {
            "humanized.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&adjusted);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, seed {variation_seed}, amount {variation_amount})",
                    text.len(),
                    adjusted.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "quantize" {
        let snapped = match quantize::quantize(&composed, grid) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use a positive finite grid in beats"),
        };
        let path = if output == "out.wav" {
            "quantized.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&snapped);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, grid {grid})",
                    text.len(),
                    snapped.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "reverse" {
        let reversed = reverse::reverse(&composed);
        let path = if output == "out.wav" {
            "reversed.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&reversed);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, reversed)",
                    text.len(),
                    reversed.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

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
        let (samples, bytes) = rendered_wav(&composed, &audio);
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
        let (samples, bytes) = rendered_wav(&composed, &audio);
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
