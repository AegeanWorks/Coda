//! Command-line front end for Coda.

use coda::accent;
use coda::analyze;
use coda::arpeggiate;
use coda::augment;
use coda::autopan;
use coda::bandpass;
use coda::bitdepth;
use coda::channels;
use coda::chord;
use coda::compare;
use coda::concat;
use coda::crush;
use coda::dc;
use coda::diatonic;
use coda::digest;
use coda::distort;
use coda::echo;
use coda::echo_wav;
use coda::extract;
use coda::fade;
use coda::fade_presets;
use coda::gain;
use coda::gate;
use coda::gen::{self, GenOptions, Scale, Style};
use coda::harmonize;
use coda::health;
use coda::highpass;
use coda::humanize;
use coda::intervals;
use coda::invert;
use coda::join;
use coda::lint;
use coda::loop_score;
use coda::lowpass;
use coda::midi_import;
use coda::midi_info;
use coda::mix;
use coda::normalize;
use coda::notes;
use coda::overlay;
use coda::pad;
use coda::pan;
use coda::pedal;
use coda::polarity;
use coda::quantize;
use coda::register;
use coda::render::{render, wav_bytes, Frame};
use coda::report;
use coda::resample;
use coda::reverse;
use coda::reverse_region;
use coda::ring;
use coda::score;
use coda::snap_scale;
use coda::speed;
use coda::splice;
use coda::stamp;
use coda::stats;
use coda::stretch;
use coda::swing;
use coda::templates;
use coda::transpose;
use coda::tremolo;
use coda::trim;
use coda::vary;
use coda::velocity;
use coda::verify;
use coda::wav_reverse;
use coda::wave::Wave;
use coda::width;
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
  coda fade-to <in.wav> [--fade-in S] [--fade-out S] [-o out.wav]
  coda loop <score.coda> --times N [-o out.coda]
  coda compare <a.coda> <b.coda>
  coda concat <a.coda> <b.coda> [...] [-o out.coda]
  coda resample <in.wav> --rate HZ [-o out.wav]
  coda channels <in.wav> --mode mono|swap|left|right [-o out.wav]
  coda bitdepth <in.wav> --bits 8|16 [-o out.wav]
  coda verify <score.coda> <in.wav> [--sample-rate N]
  coda stamp <score.coda> <in.wav> [-o out.wav]
  coda stamp-info <in.wav>
  coda report <score.coda> [--audio in.wav] [-o report.md]
  coda width <in.wav> --width 0..2 [-o out.wav]
  coda gain <in.wav> --db N [-o out.wav]
  coda tremolo <in.wav> [--lfo-rate HZ] [--lfo-depth 0..1] [-o out.wav]
  coda gate <in.wav> [--threshold DB] [-o out.wav]
  coda crush <in.wav> [--crush-bits 1..16] [--hold N] [-o out.wav]
  coda ring <a.wav> <b.wav> [-o out.wav]
  coda echo-to <in.wav> [--echo-delay S] [--echo-feedback 0..1] [-o out.wav]
  coda augment <score.coda> --factor F [-o out.coda]
  coda polarity <in.wav> [-o out.wav]
  coda overlay <a.coda> <b.coda> [...] [-o out.coda]
  coda health <in.wav>
  coda pedal <score.coda> --note C2 [--velocity 0..1] [-o out.coda]
  coda snap-scale <score.coda> --root NOTE [--scale NAME] [-o out.coda]
  coda splice <a.wav> <b.wav> --at S [-o out.wav]
  coda lowpass <in.wav> --alpha 0..1 [-o out.wav]
  coda pan <in.wav> --pan -1..1 [-o out.wav]
  coda highpass <in.wav> --alpha 0..1 [-o out.wav]
  coda speed <in.wav> --factor F [-o out.wav]
  coda velocity <score.coda> --peak 0..1 [-o out.coda]
  coda midi-info <file.mid>
  coda chord <score.coda> --at B --pitch NOTE --quality NAME --beats B [-o out.coda]
  coda distort <in.wav> --drive 0..1 [-o out.wav]
  coda bandpass <in.wav> --high-alpha 0..1 --low-alpha 0..1 [-o out.wav]
  coda intervals <score.coda>
  coda reverse-region <in.wav> --start S --end S [-o out.wav]
  coda dc <in.wav> [-o out.wav]
  coda pad <in.wav> [--before S] [--after S] [-o out.wav]
  coda harmonize <score.coda> --intervals 4,7 [-o out.coda]
  coda arpeggiate <score.coda> [--grid 0.25] [-o out.coda]
  coda invert <score.coda> [--center C4] [-o out.coda]
  coda extract <score.coda> --start B --end B [-o out.coda]
  coda accent <score.coda> --pattern 1.0,0.5 [-o out.coda]
  coda register <score.coda> --low C4 --high C5 [-o out.coda]
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
      --lfo-rate HZ    Tremolo LFO rate in Hz (default: 5)
      --lfo-depth 0..1 Tremolo depth (default: 0.5)
      --crush-bits N   Effective bit depth 1..16 for crush (default: 16)
      --hold N         Sample-and-hold factor for crush (default: 1)
      --grid B         Quantization grid in beats (default: 0.25)
      --gain-b 0..1    Gain on the second WAV in mix (default: 0.5)
      --steps N        Scale degrees to move in diatonic (default: 0)
      --ratio 0..1     Swing amount (default: 0.6)
      --factor F       Time-stretch factor (default: 1)
      --crossfade S    Overlap between joined sections in seconds (default: 0)
      --start S / --end S  Time range for trim / reverse-region
      --trim-silence   Auto-trim silence below --threshold
      --threshold DB   Silence threshold for trim and gate (default: -60 dBFS)
      --preset NAME    Fade preset: short|gentle|long|tail|attack
      --times N        Repetitions for loop (default: 1)
      --rate HZ        Target sample rate for resample
      --mode NAME      Channel mode: mono|swap|left|right
      --bits N         Output bit depth for bitdepth: 8 or 16
      --bars N         Bars to generate for `seed` (default: 4)
      --tracks N       Tracks to write for `album` (default: 3)
      --wave <name>    sine | square | triangle | saw | noise | pulse | organ | pluck
      --scale <name>   major | minor | pentatonic | chromatic (seed only)
      --root <note>    Root note for seed (default: C4)
      --time-signature N/D  Meter for MIDI export (e.g. 3/4)
      --audio FILE     WAV to include in report
      --width 0..2     Stereo width for width (0 mono, 1 identity)
      --db N           Gain in dB for gain (-100..6)
      --before S / --after S  Silence seconds for pad
      --at S           Time position for splice (seconds) or chord (beats)
      --pitch NOTE     Root pitch for chord
      --quality NAME   major | minor | dim | aug for chord
      --beats B        Duration of the chord triad
      --drive 0..1     Saturation amount for distort (0 = identity)
      --high-alpha N   High-pass coefficient for bandpass
      --low-alpha N    Low-pass coefficient for bandpass
      --alpha 0..1     Filter coefficient (lowpass 1 = identity, highpass 1 = first difference)
      --pan -1..1      Static stereo position for pan (0 = identity)
      --intervals LIST Comma-separated semitones for harmonize
      --center NOTE    Center pitch for invert (default: C4)
      --note NOTE      Pedal pitch for pedal
      --velocity 0..1  Velocity of the pedal drone (default: 0.5)
      --pattern LIST   Velocities 0..1 for accent
      --low NOTE / --high NOTE  Register bounds for register
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
    let mut factor_seen = false;
    let mut rms_target: Option<f64> = None;
    let mut crossfade = 0.0_f64;
    let mut trim_start = 0.0_f64;
    let mut trim_end: Option<f64> = None;
    let mut trim_silence = false;
    let mut trim_threshold = -60.0_f64;
    let mut fade_preset: Option<String> = None;
    let mut times = 1_usize;
    let mut target_rate: Option<u32> = None;
    let mut channel_mode: Option<String> = None;
    let mut bits: u16 = 8;
    let mut root_note: Option<String> = None;
    let mut time_signature: Option<(u8, u8)> = None;
    let mut audio_path: Option<String> = None;
    let mut width_amount = 1.0_f64;
    let mut db_gain: Option<f64> = None;
    let mut pad_before = 0.0_f64;
    let mut pad_after = 0.0_f64;
    let mut intervals: Vec<i32> = Vec::new();
    let mut center_note = "C4".to_string();
    let mut pedal_note: Option<String> = None;
    let mut pedal_velocity = 0.5_f64;
    let mut splice_at: Option<f64> = None;
    let mut lp_alpha: Option<f64> = None;
    let mut static_pan: Option<f64> = None;
    let mut accent_pattern: Vec<f64> = Vec::new();
    let mut lfo_rate = 5.0_f64;
    let mut lfo_depth = 0.5_f64;
    let mut low_note: Option<String> = None;
    let mut high_note: Option<String> = None;
    let mut crush_bits: u8 = 16;
    let mut hold: usize = 1;
    let mut tracks = 3_u32;
    let mut options = GenOptions::default();
    let mut wave_override: Option<Wave> = None;
    let mut tempo_override: Option<f64> = None;
    let mut semitones = 0_i32;
    let mut variation_seed = 0_u64;
    let mut variation_amount = 0.5_f64;
    let mut peak = 0.95_f64;
    let mut peak_seen = false;
    let mut chord_pitch: Option<String> = None;
    let mut chord_quality: Option<String> = None;
    let mut chord_beats = 1.0_f64;
    let mut drive_amount: Option<f64> = None;
    let mut high_alpha: Option<f64> = None;
    let mut low_alpha: Option<f64> = None;
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
            "--lfo-rate" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v > 0.0 && v.is_finite() => {
                    lfo_rate = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--lfo-rate must be a positive number",
                        "coda tremolo in.wav --lfo-rate 5",
                    )
                }
            },
            "--lfo-depth" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    lfo_depth = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--lfo-depth must be between 0 and 1",
                        "coda tremolo in.wav --lfo-depth 0.5",
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
                    factor_seen = true;
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
            "--mode" => match need(i) {
                Some(name) => {
                    channel_mode = Some(name.to_string());
                    i += 2;
                }
                None => return fail("--mode needs a name", "coda channels in.wav --mode mono"),
            },
            "--root" => match need(i) {
                Some(name) => {
                    root_note = Some(name.to_string());
                    i += 2;
                }
                None => return fail("--root needs a note name", "coda seed 42 --root D4"),
            },
            "--time-signature" => {
                let raw = need(i).unwrap_or("");
                let parts: Vec<&str> = raw.split('/').collect();
                let parsed = match parts.as_slice() {
                    [n, d] => n.parse::<u8>().ok().zip(d.parse::<u8>().ok()),
                    _ => None,
                };
                match parsed {
                    Some(pair @ (numerator, denominator))
                        if numerator > 0 && [1, 2, 4, 8, 16, 32].contains(&denominator) =>
                    {
                        time_signature = Some(pair);
                        i += 2;
                    }
                    _ => {
                        return fail(
                            "--time-signature must be N/D with D a power of two up to 32",
                            "coda render score.coda --time-signature 3/4 -o out.mid",
                        )
                    }
                }
            }
            "--audio" => match need(i) {
                Some(path) => {
                    audio_path = Some(path.to_string());
                    i += 2;
                }
                None => {
                    return fail(
                        "--audio needs a WAV path",
                        "coda report score.coda --audio out.wav",
                    )
                }
            },
            "--width" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=2.0).contains(&v) => {
                    width_amount = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--width must be between 0 and 2",
                        "coda width in.wav --width 1.5",
                    )
                }
            },
            "--db" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (-100.0..=6.0).contains(&v) => {
                    db_gain = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--db must be between -100 and +6",
                        "coda gain in.wav --db -6",
                    )
                }
            },
            "--before" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 && v.is_finite() => {
                    pad_before = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--before must be a non-negative number",
                        "coda pad in.wav --before 0.5",
                    )
                }
            },
            "--after" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 && v.is_finite() => {
                    pad_after = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--after must be a non-negative number",
                        "coda pad in.wav --after 0.5",
                    )
                }
            },
            "--intervals" => match need(i) {
                Some(list) => match list
                    .split(',')
                    .map(|part| part.trim().parse::<i32>())
                    .collect::<Result<Vec<i32>, _>>()
                {
                    Ok(values) => {
                        intervals = values;
                        i += 2;
                    }
                    Err(_) => {
                        return fail(
                            "--intervals must be comma-separated integers",
                            "coda harmonize score.coda --intervals 4,7",
                        )
                    }
                },
                None => {
                    return fail(
                        "--intervals needs a list",
                        "coda harmonize score.coda --intervals 4,7",
                    )
                }
            },
            "--pan" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (-1.0..=1.0).contains(&v) => {
                    static_pan = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--pan must be between -1 and 1",
                        "coda pan in.wav --pan -0.5",
                    )
                }
            },
            "--alpha" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    lp_alpha = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--alpha must be between 0 and 1",
                        "coda lowpass in.wav --alpha 0.2",
                    )
                }
            },
            "--high-alpha" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    high_alpha = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--high-alpha must be between 0 and 1",
                        "coda bandpass in.wav --high-alpha 0.1 --low-alpha 0.4",
                    )
                }
            },
            "--low-alpha" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    low_alpha = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--low-alpha must be between 0 and 1",
                        "coda bandpass in.wav --high-alpha 0.1 --low-alpha 0.4",
                    )
                }
            },
            "--drive" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    drive_amount = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--drive must be between 0 and 1",
                        "coda distort in.wav --drive 0.5",
                    )
                }
            },
            "--pitch" => match need(i) {
                Some(name) => {
                    chord_pitch = Some(name.to_string());
                    i += 2;
                }
                None => {
                    return fail(
                        "--pitch needs a note name",
                        "coda chord score.coda --at 0 --pitch C4 --quality major",
                    )
                }
            },
            "--quality" => match need(i) {
                Some(name) => {
                    chord_quality = Some(name.to_string());
                    i += 2;
                }
                None => {
                    return fail(
                        "--quality needs a name",
                        "coda chord score.coda --at 0 --pitch C4 --quality major",
                    )
                }
            },
            "--beats" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v > 0.0 && v.is_finite() => {
                    chord_beats = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--beats must be a positive number",
                        "coda chord score.coda --at 0 --pitch C4 --quality major --beats 2",
                    )
                }
            },
            "--at" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if v >= 0.0 && v.is_finite() => {
                    splice_at = Some(v);
                    i += 2;
                }
                _ => {
                    return fail(
                        "--at must be a non-negative number",
                        "coda splice a.wav b.wav --at 0.5",
                    )
                }
            },
            "--note" => match need(i) {
                Some(name) => {
                    pedal_note = Some(name.to_string());
                    i += 2;
                }
                None => {
                    return fail(
                        "--note needs a pitch name",
                        "coda pedal score.coda --note C2",
                    )
                }
            },
            "--velocity" => match need(i).and_then(|v| v.parse::<f64>().ok()) {
                Some(v) if (0.0..=1.0).contains(&v) => {
                    pedal_velocity = v;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--velocity must be between 0 and 1",
                        "coda pedal score.coda --note C2 --velocity 0.4",
                    )
                }
            },
            "--center" => match need(i) {
                Some(name) => {
                    center_note = name.to_string();
                    i += 2;
                }
                None => {
                    return fail(
                        "--center needs a note name",
                        "coda invert score.coda --center C4",
                    )
                }
            },
            "--pattern" => match need(i) {
                Some(list) => match list
                    .split(',')
                    .map(|part| part.trim().parse::<f64>())
                    .collect::<Result<Vec<f64>, _>>()
                {
                    Ok(values) => {
                        accent_pattern = values;
                        i += 2;
                    }
                    Err(_) => {
                        return fail(
                            "--pattern must be comma-separated numbers",
                            "coda accent score.coda --pattern 1.0,0.5",
                        )
                    }
                },
                None => {
                    return fail(
                        "--pattern needs a list",
                        "coda accent score.coda --pattern 1.0,0.5",
                    )
                }
            },
            "--crush-bits" => match need(i).and_then(|v| v.parse::<u8>().ok()) {
                Some(n) if (1..=16).contains(&n) => {
                    crush_bits = n;
                    i += 2;
                }
                _ => {
                    return fail(
                        "--crush-bits must be between 1 and 16",
                        "coda crush in.wav --crush-bits 8",
                    )
                }
            },
            "--hold" => match need(i).and_then(|v| v.parse::<usize>().ok()) {
                Some(n) if n >= 1 => {
                    hold = n;
                    i += 2;
                }
                _ => return fail("--hold must be at least 1", "coda crush in.wav --hold 4"),
            },
            "--low" => match need(i) {
                Some(name) => {
                    low_note = Some(name.to_string());
                    i += 2;
                }
                None => {
                    return fail(
                        "--low needs a note name",
                        "coda register score.coda --low C4 --high C5",
                    )
                }
            },
            "--high" => match need(i) {
                Some(name) => {
                    high_note = Some(name.to_string());
                    i += 2;
                }
                None => {
                    return fail(
                        "--high needs a note name",
                        "coda register score.coda --low C4 --high C5",
                    )
                }
            },
            "--bits" => match need(i).and_then(|v| v.parse::<u16>().ok()) {
                Some(n @ (8 | 16)) => {
                    bits = n;
                    i += 2;
                }
                _ => return fail("--bits must be 8 or 16", "coda bitdepth in.wav --bits 8"),
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
                    peak_seen = true;
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

    if command == "bitdepth" {
        if positional.is_empty() {
            return fail(
                "bitdepth needs an input WAV",
                "coda bitdepth in.wav --bits 8",
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
        let converted = match bitdepth::convert(&bytes, bits) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            format!("bits{bits}.wav")
        } else {
            output.clone()
        };
        return match fs::write(&path, &converted) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, {bits}-bit)", converted.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "pad" {
        if positional.is_empty() {
            return fail(
                "pad needs an input WAV",
                "coda pad in.wav --before 0.5 [--after 0.5]",
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
        let padded = match pad::pad(&bytes, pad_before, pad_after) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "padded.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &padded) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, +{pad_before}s/-{pad_after}s)",
                    padded.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "dc" {
        if positional.is_empty() {
            return fail("dc needs an input WAV", "coda dc in.wav [-o out.wav]");
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
        let cleaned = match dc::remove_dc(&bytes) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "dc.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &cleaned) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, DC removed)", cleaned.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "reverse-region" {
        if positional.is_empty() {
            return fail(
                "reverse-region needs an input WAV",
                "coda reverse-region in.wav --start 0.5 --end 1.0",
            );
        }
        let end = match trim_end {
            Some(value) => value,
            None => {
                return fail(
                    "reverse-region needs --end",
                    "coda reverse-region in.wav --start 0.5 --end 1.0",
                )
            }
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
        let reversed = match reverse_region::reverse_region(&bytes, trim_start, end) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use 0 <= start < end in seconds"),
        };
        let path = if output == "out.wav" {
            "reversed-region.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &reversed) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {trim_start}s..{end}s)",
                    reversed.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "bandpass" {
        if positional.is_empty() {
            return fail(
                "bandpass needs an input WAV",
                "coda bandpass in.wav --high-alpha 0.1 --low-alpha 0.4",
            );
        }
        let high = match high_alpha {
            Some(value) => value,
            None => {
                return fail(
                    "bandpass needs --high-alpha",
                    "coda bandpass in.wav --high-alpha 0.1 --low-alpha 0.4",
                )
            }
        };
        let low = match low_alpha {
            Some(value) => value,
            None => {
                return fail(
                    "bandpass needs --low-alpha",
                    "coda bandpass in.wav --high-alpha 0.1 --low-alpha 0.4",
                )
            }
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
        let filtered = match bandpass::apply_bandpass(&bytes, high, low) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "bandpassed.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &filtered) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, high {high}, low {low})",
                    filtered.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "distort" {
        if positional.is_empty() {
            return fail(
                "distort needs an input WAV",
                "coda distort in.wav --drive 0.5",
            );
        }
        let drive = match drive_amount {
            Some(value) => value,
            None => return fail("distort needs --drive", "coda distort in.wav --drive 0.5"),
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
        let saturated = match distort::apply_distort(&bytes, drive) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "distorted.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &saturated) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, drive {drive})", saturated.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "midi-info" {
        if positional.is_empty() {
            return fail("midi-info needs a MIDI file", "coda midi-info file.mid");
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
        return match midi_info::midi_info(&bytes) {
            Ok(report) => {
                print!("{report}");
                ExitCode::SUCCESS
            }
            Err(e) => fail(&e, "use a Standard MIDI File written by Coda"),
        };
    }

    if command == "health" {
        if positional.is_empty() {
            return fail("health needs an input WAV", "coda health in.wav");
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
        return match health::health(&bytes) {
            Ok(report) => {
                print!("{report}");
                ExitCode::SUCCESS
            }
            Err(e) => fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
    }

    if command == "polarity" {
        if positional.is_empty() {
            return fail(
                "polarity needs an input WAV",
                "coda polarity in.wav [-o out.wav]",
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
        let flipped = match polarity::invert_polarity(&bytes) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "inverted.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &flipped) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, polarity inverted)", flipped.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "echo-to" {
        if positional.is_empty() {
            return fail(
                "echo-to needs an input WAV",
                "coda echo-to in.wav --echo-delay 0.12 [--echo-feedback 0.4]",
            );
        }
        if echo_delay <= 0.0 {
            return fail(
                "echo-to needs a positive --echo-delay",
                "coda echo-to in.wav --echo-delay 0.12 [--echo-feedback 0.4]",
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
        let echoed = match echo_wav::apply_echo_wav(&bytes, echo_delay, echo_feedback) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "echoed.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &echoed) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {echo_delay}s delay, {echo_feedback} feedback)",
                    echoed.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "crush" {
        if positional.is_empty() {
            return fail(
                "crush needs an input WAV",
                "coda crush in.wav --crush-bits 8 --hold 4",
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
        let crushed = match crush::apply_crush(&bytes, crush_bits, hold) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "crushed.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &crushed) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {crush_bits} bits, hold {hold})",
                    crushed.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "gate" {
        if positional.is_empty() {
            return fail(
                "gate needs an input WAV",
                "coda gate in.wav --threshold -40",
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
        let gated = match gate::apply_gate(&bytes, trim_threshold) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "gated.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &gated) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, threshold {trim_threshold} dBFS)",
                    gated.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "tremolo" {
        if positional.is_empty() {
            return fail(
                "tremolo needs an input WAV",
                "coda tremolo in.wav --lfo-rate 5 --lfo-depth 0.5",
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
        let changed = match tremolo::apply_tremolo(&bytes, lfo_rate, lfo_depth) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "tremolo.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &changed) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {lfo_rate} Hz, depth {lfo_depth})",
                    changed.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "gain" {
        if positional.is_empty() {
            return fail("gain needs an input WAV", "coda gain in.wav --db -6");
        }
        let db = match db_gain {
            Some(value) => value,
            None => return fail("gain needs --db", "coda gain in.wav --db -6"),
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
        let changed = match gain::change_gain(&bytes, db) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "gained.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &changed) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, {db} dB)", changed.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "width" {
        if positional.is_empty() {
            return fail("width needs an input WAV", "coda width in.wav --width 0.5");
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
        let changed = match width::change_width(&bytes, width_amount) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "width.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &changed) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, width {width_amount})",
                    changed.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "channels" {
        if positional.is_empty() {
            return fail(
                "channels needs an input WAV",
                "coda channels in.wav --mode mono",
            );
        }
        let mode = match &channel_mode {
            Some(m) => m.clone(),
            None => return fail("channels needs --mode", "coda channels in.wav --mode mono"),
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
        let changed = match channels::transform(&bytes, &mode) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use mono, swap, left or right"),
        };
        let path = if output == "out.wav" {
            "channels.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &changed) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, mode {mode})", changed.len());
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

    if command == "fade-to" {
        if positional.is_empty() {
            return fail(
                "fade-to needs an input WAV",
                "coda fade-to in.wav --fade-in 0.1 [--fade-out 0.2]",
            );
        }
        if fade_in == 0.0 && fade_out == 0.0 {
            return fail(
                "fade-to needs a non-zero duration",
                "coda fade-to in.wav --fade-in 0.1 [--fade-out 0.2]",
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
        let faded = match fade_presets::apply_seconds(&bytes, fade_in, fade_out) {
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
                println!(
                    "wrote {path} ({} bytes, in {fade_in}s, out {fade_out}s)",
                    faded.len()
                );
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

    if command == "speed" {
        if positional.is_empty() {
            return fail("speed needs an input WAV", "coda speed in.wav --factor 2");
        }
        if !factor_seen {
            return fail("speed needs --factor", "coda speed in.wav --factor 2");
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
        let changed = match speed::apply_speed(&bytes, factor) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "sped.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &changed) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} frames, factor {factor})",
                    changed.len(),
                    (changed.len() - 44) / 4
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "highpass" {
        if positional.is_empty() {
            return fail(
                "highpass needs an input WAV",
                "coda highpass in.wav --alpha 0.2",
            );
        }
        let alpha = match lp_alpha {
            Some(value) => value,
            None => return fail("highpass needs --alpha", "coda highpass in.wav --alpha 0.2"),
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
        let filtered = match highpass::apply_highpass(&bytes, alpha) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "highpassed.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &filtered) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, alpha {alpha})", filtered.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "pan" {
        if positional.is_empty() {
            return fail("pan needs an input WAV", "coda pan in.wav --pan -0.5");
        }
        let position = match static_pan {
            Some(value) => value,
            None => return fail("pan needs --pan", "coda pan in.wav --pan -0.5"),
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
        let panned = match pan::apply_pan(&bytes, position) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "panned.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &panned) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, pan {position})", panned.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "lowpass" {
        if positional.is_empty() {
            return fail(
                "lowpass needs an input WAV",
                "coda lowpass in.wav --alpha 0.2",
            );
        }
        let alpha = match lp_alpha {
            Some(value) => value,
            None => return fail("lowpass needs --alpha", "coda lowpass in.wav --alpha 0.2"),
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
        let filtered = match lowpass::apply_lowpass(&bytes, alpha) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "lowpassed.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &filtered) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, alpha {alpha})", filtered.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "splice" {
        if positional.len() < 2 {
            return fail(
                "splice needs two WAV files",
                "coda splice a.wav b.wav --at 0.5 [-o out.wav]",
            );
        }
        let at = match splice_at {
            Some(value) => value,
            None => return fail("splice needs --at", "coda splice a.wav b.wav --at 0.5"),
        };
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
        let joined = match splice::splice_wavs(&inputs[0], &inputs[1], at) {
            Ok(bytes) => bytes,
            Err(e) => return fail(&e, "both inputs must be Coda WAVs at the same sample rate"),
        };
        let path = if output == "out.wav" {
            "spliced.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &joined) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} frames, at {at}s)",
                    joined.len(),
                    (joined.len() - 44) / 4
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "ring" {
        if positional.len() < 2 {
            return fail(
                "ring needs two WAV files",
                "coda ring a.wav b.wav [-o out.wav]",
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
        let product = match ring::ring_wavs(&inputs[0], &inputs[1]) {
            Ok(bytes) => bytes,
            Err(e) => return fail(&e, "both inputs must be Coda WAVs at the same sample rate"),
        };
        let path = if output == "out.wav" {
            "ring.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &product) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} frames)",
                    product.len(),
                    (product.len() - 44) / 4
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
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

    if command == "overlay" {
        if positional.len() < 2 {
            return fail(
                "overlay needs at least two score files",
                "coda overlay a.coda b.coda [-o out.coda]",
            );
        }
        let mut scores = Vec::new();
        for path in &positional {
            match read_score(path) {
                Ok(s) => scores.push(s),
                Err(code) => return code,
            }
        }
        let layered = match overlay::overlay_scores(&scores) {
            Ok(s) => s,
            Err(e) => return fail(&e, "provide at least two valid scores"),
        };
        let path = if output == "out.wav" {
            "overlay.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&layered);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, {} scores)",
                    text.len(),
                    layered.notes.len(),
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

    if command == "stamp-info" {
        if positional.is_empty() {
            return fail("stamp-info needs a WAV", "coda stamp-info in.wav");
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
        return match stamp::read(&bytes) {
            Ok(Some(text)) => {
                println!("{text}");
                ExitCode::SUCCESS
            }
            Ok(None) => {
                println!("no CODA stamp");
                ExitCode::SUCCESS
            }
            Err(e) => fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
    }

    if command == "stamp" {
        if positional.len() < 2 {
            return fail(
                "stamp needs a score and a WAV",
                "coda stamp score.coda in.wav",
            );
        }
        let source = match read_score(&positional[0]) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let bytes = match fs::read(&positional[1]) {
            Ok(b) => b,
            Err(e) => {
                return fail(
                    &format!("cannot read '{}': {e}", positional[1]),
                    "check that the WAV file exists and is readable",
                )
            }
        };
        let canonical = score::serialize(&source);
        let payload = format!(
            "digest:{}:notes:{}:tempo:{}",
            digest::fingerprint(canonical.as_bytes()),
            source.notes.len(),
            source.tempo
        );
        let stamped = match stamp::stamp(&bytes, &payload) {
            Ok(b) => b,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "stamped.wav".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, &stamped) {
            Ok(()) => {
                println!("wrote {path} ({} bytes, stamped)", stamped.len());
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
        | "diatonic" | "swing" | "stretch" | "loop" | "reverse" | "quantize" | "verify"
        | "report" | "harmonize" | "arpeggiate" | "invert" | "extract" | "accent" | "register"
        | "augment" | "pedal" | "snap-scale" | "velocity" | "chord" | "intervals" | "digest" => {
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
            let mut generated = gen::generate(number, options);
            if let Some(name) = &root_note {
                match notes::midi(name) {
                    Some(midi) => {
                        generated = transpose::transpose(&generated, midi - 60);
                    }
                    None => {
                        return fail(
                            &format!("invalid root note '{name}'"),
                            "use a note name like C4, D4 or Bb3",
                        )
                    }
                }
            }
            generated
        }
        other => return fail(&format!("unknown command '{other}'"), "coda help"),
    };

    let composed = if semitones == 0 {
        composed
    } else {
        transpose::transpose(&composed, semitones)
    };

    if command == "chord" {
        let at = match splice_at {
            Some(value) => value,
            None => {
                return fail(
                    "chord needs --at",
                    "coda chord score.coda --at 0 --pitch C4 --quality major",
                )
            }
        };
        let pitch = match chord_pitch {
            Some(name) => name,
            None => {
                return fail(
                    "chord needs --pitch",
                    "coda chord score.coda --at 0 --pitch C4 --quality major",
                )
            }
        };
        let quality = match chord_quality {
            Some(name) => name,
            None => {
                return fail(
                    "chord needs --quality",
                    "coda chord score.coda --at 0 --pitch C4 --quality major",
                )
            }
        };
        let voiced = match chord::add_chord(&composed, at, &pitch, &quality, chord_beats) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use major, minor, dim or aug with a valid root"),
        };
        let path = if output == "out.wav" {
            "chord.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&voiced);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, {pitch} {quality} at {at})",
                    text.len(),
                    voiced.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "velocity" {
        if !peak_seen {
            return fail("velocity needs --peak", "coda velocity score.coda --peak 1");
        }
        let scaled = match velocity::normalize_velocity(&composed, peak) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use a peak between 0 and 1"),
        };
        let path = if output == "out.wav" {
            "velocity.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&scaled);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, peak {peak})",
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

    if command == "snap-scale" {
        let root = match root_note {
            Some(name) => name,
            None => {
                return fail(
                    "snap-scale needs --root",
                    "coda snap-scale score.coda --root C4 [--scale major]",
                )
            }
        };
        let snapped = match snap_scale::snap_scale(&composed, &root, options.scale) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use a note name like C4 for --root"),
        };
        let path = if output == "out.wav" {
            "snapped.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&snapped);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, {} on {root})",
                    text.len(),
                    snapped.notes.len(),
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

    if command == "pedal" {
        let note = match pedal_note {
            Some(name) => name,
            None => return fail("pedal needs --note", "coda pedal score.coda --note C2"),
        };
        let grounded = match pedal::pedal(&composed, &note, pedal_velocity) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use a note name like C2 and velocity 0..1"),
        };
        let path = if output == "out.wav" {
            "pedal.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&grounded);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, pedal {note} @ {pedal_velocity})",
                    text.len(),
                    grounded.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "augment" {
        if !factor_seen {
            return fail(
                "augment needs --factor",
                "coda augment score.coda --factor 2",
            );
        }
        let grown = match augment::augment(&composed, factor) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use a positive finite factor"),
        };
        let path = if output == "out.wav" {
            "augmented.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&grown);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, factor {factor})",
                    text.len(),
                    grown.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "register" {
        let low = match low_note {
            Some(name) => name,
            None => {
                return fail(
                    "register needs --low",
                    "coda register score.coda --low C4 --high C5",
                )
            }
        };
        let high = match high_note {
            Some(name) => name,
            None => {
                return fail(
                    "register needs --high",
                    "coda register score.coda --low C4 --high C5",
                )
            }
        };
        let fitted = match register::register(&composed, &low, &high) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use note names like C4 and C5 with low <= high"),
        };
        let path = if output == "out.wav" {
            "registered.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&fitted);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, {low}..{high})",
                    text.len(),
                    fitted.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "accent" {
        let accented = match accent::accent(&composed, &accent_pattern) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use velocities between 0 and 1"),
        };
        let path = if output == "out.wav" {
            "accented.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&accented);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, {} velocities)",
                    text.len(),
                    accented.notes.len(),
                    accent_pattern.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "extract" {
        let end = match trim_end {
            Some(value) => value,
            None => {
                return fail(
                    "extract needs --end",
                    "coda extract score.coda --start 0 --end 4",
                )
            }
        };
        let cut = match extract::extract(&composed, trim_start, end) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use 0 <= start < end in beats"),
        };
        let path = if output == "out.wav" {
            "extracted.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&cut);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, {trim_start}..{end})",
                    text.len(),
                    cut.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "invert" {
        let mirrored = match invert::invert(&composed, &center_note) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use a note name like C4 or Bb3"),
        };
        let path = if output == "out.wav" {
            "inverted.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&mirrored);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, center {center_note})",
                    text.len(),
                    mirrored.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "arpeggiate" {
        let rolled = match arpeggiate::arpeggiate(&composed, grid) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use a positive finite grid in beats"),
        };
        let path = if output == "out.wav" {
            "arpeggiated.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&rolled);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, grid {grid})",
                    text.len(),
                    rolled.notes.len()
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "harmonize" {
        let harmonized = match harmonize::harmonize(&composed, &intervals) {
            Ok(s) => s,
            Err(e) => return fail(&e, "use semitone intervals between -24 and 24"),
        };
        let path = if output == "out.wav" {
            "harmonized.coda".to_string()
        } else {
            output.clone()
        };
        let text = score::serialize(&harmonized);
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!(
                    "wrote {path} ({} bytes, {} notes, {} voices added)",
                    text.len(),
                    harmonized.notes.len(),
                    harmonized.notes.len().saturating_sub(composed.notes.len())
                );
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "report" {
        let audio = match &audio_path {
            Some(path) => match fs::read(path) {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    return fail(
                        &format!("cannot read '{path}': {e}"),
                        "check that the WAV file exists and is readable",
                    )
                }
            },
            None => None,
        };
        let text = match report::report(&composed, audio.as_deref()) {
            Ok(text) => text,
            Err(e) => return fail(&e, "use a 16-bit stereo WAV rendered by Coda"),
        };
        let path = if output == "out.wav" {
            "report.md".to_string()
        } else {
            output.clone()
        };
        return match fs::write(&path, text.as_bytes()) {
            Ok(()) => {
                println!("wrote {path} ({} bytes)", text.len());
                ExitCode::SUCCESS
            }
            Err(e) => fail(
                &format!("cannot write '{path}': {e}"),
                "check that the output directory exists and is writable",
            ),
        };
    }

    if command == "verify" {
        if positional.len() < 2 {
            return fail(
                "verify needs a score and a WAV",
                "coda verify score.coda audio.wav",
            );
        }
        let actual = match fs::read(&positional[1]) {
            Ok(b) => b,
            Err(e) => {
                return fail(
                    &format!("cannot read '{}': {e}", positional[1]),
                    "check that the WAV file exists and is readable",
                )
            }
        };
        let (_, expected) = rendered_wav(&composed, &audio);
        let fingerprint = digest::fingerprint(&expected);
        return match verify::first_difference(&expected, &actual) {
            None => {
                println!("OK {fingerprint} ({} bytes)", expected.len());
                ExitCode::SUCCESS
            }
            Some(offset) => {
                println!(
                    "MISMATCH at byte {offset} (expected {}, actual {})",
                    expected.get(offset).copied().unwrap_or_default(),
                    actual.get(offset).copied().unwrap_or_default()
                );
                ExitCode::from(1)
            }
        };
    }

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

    if command == "intervals" {
        return match intervals::intervals(&composed) {
            Ok(report) => {
                print!("{report}");
                ExitCode::SUCCESS
            }
            Err(e) => fail(&e, "write at least one pitched note in the score"),
        };
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
        let bytes = coda::midi::midi_bytes_ex(&composed, time_signature);
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
