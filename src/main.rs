/* use  **************************************************************************************************/

use clap::{Args, Parser, Subcommand};
use color_eyre::eyre::Result;
use crossterm::{
  cursor::{Hide, MoveTo, Show},
  event::{self, Event, KeyCode, KeyModifiers},
  execute,
  terminal::{self, Clear, ClearType},
};
use std::{
  env,
  io::{self, Write},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  time::{Duration, Instant},
};
use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, prelude::*};

/* mod  **************************************************************************************************/

/* type alias  *******************************************************************************************/

/* global const  *****************************************************************************************/

const TICK_MS: u64 = 20;
const HEADER_LINES: usize = 7;
const FLOOR_LINES: usize = 1;
const POSE_LINES: usize = 5;
const POSE_COUNT: usize = 9;
const DEFAULT_ROWS: usize = 24;
const FLOOR: &str = "==============================";
const POSES: [[&str; POSE_LINES]; POSE_COUNT] = [
  ["   O   ", "  /|\\  ", "   |   ", "  / \\  ", " /   \\ "],
  ["   O   ", "  /|\\  ", "   |   ", "  / \\  ", " /_ _\\ "],
  ["   O   ", "  /|\\  ", "   |   ", "  /_\\  ", " /   \\ "],
  ["   O   ", "  /|\\  ", "  _|_  ", "  /_\\  ", " /   \\ "],
  ["   O   ", "  /|\\  ", "  _|_  ", "  /_\\  ", " _/ \\_ "],
  ["   O   ", "  /|\\  ", "  _|_  ", " _/_\\_ ", " _/ \\_ "],
  ["   O   ", "  /|\\  ", " __|__ ", " _/_\\_ ", " _/ \\_ "],
  ["   O   ", " _/|\\_ ", " __|__ ", " _/_\\_ ", " _/ \\_ "],
  ["   O   ", " _/|\\_ ", " __|__ ", " _/_\\_ ", "__/ \\__"],
];

/* trait  ************************************************************************************************/

/* enum  *************************************************************************************************/

#[derive(Subcommand, Debug)]
enum Commands {
  Squat(SquatArgs),
}

#[derive(Debug)]
enum InputAction {
  None,
  TogglePause,
  Exit,
}

/* struct  ***********************************************************************************************/

#[derive(Parser, Debug)]
#[command(name = "trainer", version, about = "CLI training utilities")]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Args, Debug)]
struct SquatArgs {
  #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(1..))]
  duration: u64,
  #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..))]
  count: u32,
  #[arg(long, default_value_t = 3, value_parser = clap::value_parser!(u64).range(0..))]
  countdown: u64,
}

struct FrameState<'a> {
  current: u32,
  total: u32,
  phase: &'a str,
  half_secs: f64,
  remaining: Duration,
  paused: bool,
  offset: usize,
  pose_idx: usize,
  max_drop_lines: usize,
  stretch: f64,
}

struct TerminalGuard;

/* unsafe impl standard traits  **************************************************************************/

/* impl standard traits  *********************************************************************************/

impl TerminalGuard {
  fn new() -> Result<Self> {
    terminal::enable_raw_mode()?;
    execute!(io::stdout(), Hide)?;
    Ok(Self)
  }
}

impl Drop for TerminalGuard {
  fn drop(&mut self) {
    let _ = execute!(io::stdout(), Show);
    let _ = terminal::disable_raw_mode();
  }
}

/* impl custom traits  ***********************************************************************************/

/* impl  *************************************************************************************************/

/* fn  ***************************************************************************************************/

fn init_tracing() -> Result<()> {
  if env::var("RUST_LOG").is_err() {
    unsafe {
      env::set_var("RUST_LOG", "info");
    }
  }

  let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

  tracing_subscriber::registry()
    .with(env_filter)
    .with(tracing_subscriber::fmt::layer())
    .with(ErrorLayer::default())
    .try_init()?;

  Ok(())
}

fn format_mmss_millis(duration: Duration) -> String {
  let total_secs = duration.as_secs();
  let minutes = total_secs / 60;
  let seconds = total_secs % 60;
  let millis = duration.subsec_millis();
  format!("{:02}:{:02}.{:03}", minutes, seconds, millis)
}

fn terminal_rows() -> usize {
  terminal::size()
    .map(|(_, rows)| rows as usize)
    .unwrap_or(DEFAULT_ROWS)
}

fn read_input(timeout: Duration) -> Result<InputAction> {
  if !event::poll(timeout)? {
    return Ok(InputAction::None);
  }

  match event::read()? {
    Event::Key(key) => match key.code {
      KeyCode::Esc => Ok(InputAction::Exit),
      KeyCode::Char(' ') => Ok(InputAction::TogglePause),
      KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Ok(InputAction::Exit),
      _ => Ok(InputAction::None),
    },
    _ => Ok(InputAction::None),
  }
}

fn build_figure_lines(offset: usize, pose_idx: usize, max_drop_lines: usize) -> Vec<String> {
  let mut lines = Vec::new();
  let clamped_offset = offset.min(max_drop_lines);
  lines.extend(std::iter::repeat(String::new()).take(clamped_offset));
  let pose = &POSES[pose_idx.min(POSE_COUNT - 1)];
  lines.extend(pose.iter().map(|line| (*line).to_string()));

  let total_body = max_drop_lines + POSE_LINES;
  let current_body = clamped_offset + POSE_LINES;
  if total_body > current_body {
    lines.extend(std::iter::repeat(String::new()).take(total_body - current_body));
  }

  lines.push(FLOOR.to_string());
  lines
}

fn draw_frame(stdout: &mut io::Stdout, state: &FrameState) -> Result<()> {
  execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;

  let status = if state.paused { "PAUSED" } else { "RUNNING" };
  let mut output = String::new();
  output.push_str(&format!(
    "Slow Squat  Rep: {}/{}\r\n",
    state.current, state.total
  ));
  output.push_str(&format!(
    "Phase: {}  Tempo: down {:.1}s / up {:.1}s\r\n",
    state.phase, state.half_secs, state.half_secs
  ));
  output.push_str(&format!("伸長(100=伸,0=縮): {:.1}\r\n", state.stretch));
  output.push_str(&format!(
    "Time left: {}\r\n",
    format_mmss_millis(state.remaining)
  ));
  output.push_str(&format!("Status: {}\r\n", status));
  output.push_str("Controls: SPACE=Pause/Resume  ESC=Quit  Ctrl+C=Quit\r\n\r\n");

  let figure_lines = build_figure_lines(state.offset, state.pose_idx, state.max_drop_lines);
  for (idx, line) in figure_lines.iter().enumerate() {
    output.push_str(line);
    if idx + 1 < figure_lines.len() {
      output.push_str("\r\n");
    }
  }

  write!(stdout, "{}", output)?;
  stdout.flush()?;
  Ok(())
}

fn draw_message(stdout: &mut io::Stdout, message: &str, line2: &str) -> Result<()> {
  execute!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
  write!(stdout, "{}\r\n", message)?;
  if !line2.is_empty() {
    write!(stdout, "{}\r\n", line2)?;
  }
  stdout.flush()?;
  Ok(())
}

fn run_countdown(stdout: &mut io::Stdout, seconds: u64, exit_flag: &AtomicBool) -> Result<bool> {
  if seconds == 0 {
    return Ok(true);
  }

  for remaining in (1..=seconds).rev() {
    draw_message(stdout, "Starting in...", &format!("{}", remaining))?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(1) {
      if exit_flag.load(Ordering::SeqCst) {
        return Ok(false);
      }
      match read_input(Duration::from_millis(50))? {
        InputAction::Exit => return Ok(false),
        _ => {}
      }
    }
  }

  Ok(true)
}

fn run_squat(args: SquatArgs) -> Result<()> {
  let exit_flag = Arc::new(AtomicBool::new(false));
  let exit_flag_clone = exit_flag.clone();
  ctrlc::set_handler(move || {
    exit_flag_clone.store(true, Ordering::SeqCst);
  })?;

  let _terminal_guard = TerminalGuard::new()?;
  let mut stdout = io::stdout();

  if !run_countdown(&mut stdout, args.countdown, &exit_flag)? {
    draw_message(&mut stdout, "Stopped.", &format!("Reps: 0/{}", args.count))?;
    return Ok(());
  }

  let total_duration = Duration::from_secs(args.duration);
  let rep_duration = total_duration.as_secs_f64() / args.count as f64;
  let half_duration = rep_duration / 2.0;

  let mut paused = false;
  let mut paused_at: Option<Instant> = None;
  let mut paused_total = Duration::ZERO;
  let start = Instant::now();

  let mut aborted = false;
  let mut completed_reps = 0;

  loop {
    if exit_flag.load(Ordering::SeqCst) {
      aborted = true;
      break;
    }

    match read_input(Duration::from_millis(TICK_MS))? {
      InputAction::Exit => {
        aborted = true;
        break;
      }
      InputAction::TogglePause => {
        if paused {
          if let Some(paused_start) = paused_at.take() {
            paused_total =
              paused_total.saturating_add(Instant::now().saturating_duration_since(paused_start));
          }
          paused = false;
        } else {
          paused = true;
          paused_at = Some(Instant::now());
        }
      }
      InputAction::None => {}
    }

    let now = Instant::now();
    let effective_now = if paused {
      paused_at.unwrap_or(now)
    } else {
      now
    };
    let elapsed = effective_now
      .saturating_duration_since(start)
      .saturating_sub(paused_total);

    if elapsed >= total_duration {
      break;
    }

    let elapsed_secs = elapsed.as_secs_f64();
    let rep_index = (elapsed_secs / rep_duration).floor() as u32;
    let completed = rep_index.min(args.count);
    completed_reps = completed;
    let within_rep = elapsed_secs - (rep_index as f64 * rep_duration);

    let (phase, progress) = if within_rep < half_duration {
      ("DOWN", within_rep / half_duration)
    } else {
      ("UP", 1.0 - (within_rep - half_duration) / half_duration)
    };

    let clamped = progress.clamp(0.0, 1.0);
    let max_drop_lines = terminal_rows().saturating_sub(HEADER_LINES + POSE_LINES + FLOOR_LINES);
    let offset = (clamped * max_drop_lines as f64)
      .round()
      .min(max_drop_lines as f64) as usize;
    let pose_idx = (clamped * (POSE_COUNT.saturating_sub(1)) as f64).round() as usize;
    let stretch = (1.0 - clamped) * 100.0;

    let remaining = total_duration.saturating_sub(elapsed);
    let current_rep = (completed.saturating_add(1)).min(args.count);
    let state = FrameState {
      current: current_rep,
      total: args.count,
      phase,
      half_secs: half_duration,
      remaining,
      paused,
      offset,
      pose_idx,
      max_drop_lines,
      stretch,
    };

    draw_frame(&mut stdout, &state)?;
  }

  if aborted {
    draw_message(
      &mut stdout,
      "Stopped.",
      &format!("Reps: {}/{}", completed_reps, args.count),
    )?;
  } else {
    draw_message(
      &mut stdout,
      "Complete!",
      &format!("Reps: {}/{}", args.count, args.count),
    )?;
  }

  Ok(())
}

fn main() -> Result<()> {
  color_eyre::install()?;
  init_tracing()?;

  let cli = Cli::parse();

  match cli.command {
    Commands::Squat(args) => run_squat(args),
  }
}

/* async fn  *********************************************************************************************/

/* test for pri ******************************************************************************************/

/* test for pub ******************************************************************************************/
