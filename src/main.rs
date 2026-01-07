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
use tiny_http::{Header, Response, Server};
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
const HOLD_SECS: f64 = 5.0;
const FLOOR: &str = "==============================";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
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
const SQUAT_WEB_HTML: &str = r##"<!doctype html>
<html lang="ja">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Slow Squat</title>
    <style>
      :root {
        --bg: #f5f0e6;
        --ink: #1d1c1a;
        --accent: #c24a3a;
        --accent-2: #2f6f6d;
        --grid: #e1d6c4;
        --paper: rgba(255, 255, 255, 0.78);
        --shadow: 0 18px 50px rgba(36, 32, 27, 0.18);
      }
      * {
        box-sizing: border-box;
      }
      body {
        margin: 0;
        min-height: 100vh;
        font-family: "Hiragino Sans", "Avenir Next", "Yu Gothic", "YuGothic",
          "Helvetica Neue", sans-serif;
        background: radial-gradient(
            900px 500px at 85% 10%,
            rgba(47, 111, 109, 0.18),
            transparent 60%
          ),
          radial-gradient(
            800px 500px at 10% 15%,
            rgba(194, 74, 58, 0.2),
            transparent 65%
          ),
          var(--bg);
        color: var(--ink);
      }
      body::before,
      body::after {
        content: "";
        position: fixed;
        inset: auto;
        width: 240px;
        height: 240px;
        border-radius: 50%;
        filter: blur(40px);
        opacity: 0.35;
        pointer-events: none;
        z-index: 0;
        animation: float 14s ease-in-out infinite;
      }
      body::before {
        top: -60px;
        right: -40px;
        background: rgba(194, 74, 58, 0.45);
      }
      body::after {
        bottom: -80px;
        left: -60px;
        background: rgba(47, 111, 109, 0.4);
        animation-delay: -7s;
      }
      @keyframes float {
        0%,
        100% {
          transform: translateY(0) translateX(0);
        }
        50% {
          transform: translateY(-18px) translateX(10px);
        }
      }
      #version {
        position: fixed;
        top: 10px;
        right: 12px;
        font-size: 12px;
        letter-spacing: 0.04em;
        opacity: 0.6;
        pointer-events: none;
        z-index: 3;
      }
      #app {
        min-height: 100vh;
        display: flex;
        flex-direction: column;
        gap: 16px;
        padding: clamp(16px, 3vw, 28px);
        position: relative;
        z-index: 1;
      }
      #info {
        padding: 18px 22px 14px;
        line-height: 1.6;
        background: var(--paper);
        border: 1px solid var(--grid);
        border-radius: 18px;
        box-shadow: var(--shadow);
        backdrop-filter: blur(12px);
        position: relative;
        overflow: hidden;
        animation: rise 0.8s ease-out both;
      }
      #info::before {
        content: "";
        position: absolute;
        inset: 0;
        background: linear-gradient(
          120deg,
          rgba(194, 74, 58, 0.12),
          transparent 45%,
          rgba(47, 111, 109, 0.1)
        );
        opacity: 0.6;
        pointer-events: none;
      }
      #line1 {
        font-size: clamp(18px, 2.4vw, 22px);
        font-weight: 700;
        letter-spacing: 0.06em;
        text-transform: uppercase;
      }
      #line2 {
        font-size: clamp(14px, 2vw, 16px);
        opacity: 0.85;
      }
      #line4,
      #line5,
      #line6 {
        font-size: clamp(12px, 1.6vw, 14px);
      }
      #line6 {
        opacity: 0.7;
        letter-spacing: 0.02em;
      }
      #settings {
        margin-top: 8px;
        display: flex;
        align-items: center;
        gap: 12px;
        font-size: 12px;
        letter-spacing: 0.04em;
        opacity: 0.85;
      }
      #settings label {
        display: inline-flex;
        align-items: center;
        gap: 8px;
        font-weight: 600;
      }
      #settings input[type="checkbox"] {
        width: 16px;
        height: 16px;
        accent-color: var(--accent);
      }
      #canvas-wrap {
        flex: 1;
        display: flex;
        min-height: 280px;
        background: var(--paper);
        border: 1px solid var(--grid);
        border-radius: 22px;
        box-shadow: var(--shadow);
        overflow: hidden;
        position: relative;
        align-items: stretch;
        animation: rise 0.9s ease-out both;
        animation-delay: 0.08s;
      }
      #canvas-wrap::after {
        content: "";
        position: absolute;
        inset: 0;
        background: linear-gradient(
          140deg,
          rgba(255, 255, 255, 0.4),
          transparent 40%,
          rgba(47, 111, 109, 0.08)
        );
        pointer-events: none;
        z-index: 1;
      }
      canvas {
        position: absolute;
        inset: 0;
        width: 100%;
        height: 100%;
        display: block;
        z-index: 0;
      }
      @keyframes rise {
        0% {
          opacity: 0;
          transform: translateY(12px);
        }
        100% {
          opacity: 1;
          transform: translateY(0);
        }
      }
      @media (max-width: 700px) {
        #info {
          padding: 16px 18px 12px;
        }
        #canvas-wrap {
          min-height: 240px;
        }
      }
    </style>
  </head>
  <body>
    <div id="version">v__VERSION__</div>
    <div id="app">
      <div id="info">
        <div id="line1">Slow Squat  Set: 1/__SETS__  Rep: 1/__COUNT__</div>
        <div id="line2">Phase: DOWN  Tempo: down 0.0s / hold 0.0s / up 0.0s</div>
        <div id="line4">Time left: 00:00.000</div>
        <div id="line5">Status: RUNNING</div>
        <div id="line6">Controls: SPACE=Pause/Resume  ESC=Quit  Ctrl+C=Quit</div>
        <div id="settings">
          <label>
            <input id="voice-toggle" type="checkbox" />
            Voice
          </label>
          <span>DOWN / HOLD / UP</span>
        </div>
      </div>
      <div id="canvas-wrap">
        <canvas id="squat"></canvas>
      </div>
    </div>
    <script>
      (() => {
        const config = {
          duration: __DURATION__,
          count: __COUNT__,
          sets: __SETS__,
          interval: __INTERVAL__,
          swingStart: __SWING_START__,
          swingStop: __SWING_STOP__,
          freq: __FREQ__,
        };
        const total = config.duration;
        const count = config.count;
        const sets = config.sets;
        const interval = config.interval;
        const repDuration = total / count;
        const hold = __HOLD__;
        const moveDuration = (repDuration - hold) / 2;
        const down = moveDuration;
        const up = moveDuration;
        const overallTotal = total * sets + interval * (sets - 1);
        const swingStart = config.swingStart;
        const swingStop = config.swingStop;
        const freq = config.freq;
        const isTouch =
          "ontouchstart" in window || (navigator.maxTouchPoints || 0) > 0;
        const supportsPointer = "PointerEvent" in window;
        const palette = {
          ink: "#1d1c1a",
          inkSoft: "rgba(29, 28, 26, 0.65)",
          accent: "#c24a3a",
          accent2: "#2f6f6d",
          paper: "rgba(255, 255, 255, 0.86)",
          paperStrong: "rgba(255, 255, 255, 0.95)",
          grid: "rgba(29, 28, 26, 0.08)",
        };
        const fontSans =
          '"Hiragino Sans", "Avenir Next", "Yu Gothic", "YuGothic", "Helvetica Neue", sans-serif';
        const fontMono = '"SF Mono", "Menlo", "Consolas", monospace';

        const line1 = document.getElementById("line1");
        const line2 = document.getElementById("line2");
        const line4 = document.getElementById("line4");
        const line5 = document.getElementById("line5");
        const voiceToggle = document.getElementById("voice-toggle");

        const canvas = document.getElementById("squat");
        const ctx = canvas.getContext("2d");
        let viewWidth = 0;
        let viewHeight = 0;

        let paused = false;
        let stopped = false;
        let pauseStarted = null;
        let pausedTotal = 0;
        let currentProgress = 0;
        let lastMoveProgress = 0;
        let lastHoldProgress = 0;
        let lastTimeLeft = "00:00.000";
        let lastOverallProgress = 0;
        let lastSetProgress = 0;
        let lastRestProgress = 0;
        let restActive = false;
        let tremorTime = 0;
        const countdownSeconds = 5;
        let countdownStarted = false;
        let countdownStart = null;
        let started = false;
        let animationStart = null;
        const calloutDurationMs = 700;
        let calloutText = "";
        let calloutStart = 0;
        let calloutUntil = 0;
        let lastPhase = "";
        const voiceStorageKey = "squatVoiceEnabled";
        let voiceEnabled = true;
        let speechReady = false;
        let availableVoices = [];
        let lastCountdownSpoken = null;

        try {
          const stored = localStorage.getItem(voiceStorageKey);
          if (stored !== null) {
            voiceEnabled = stored === "1";
          }
        } catch {}
        if (voiceToggle) {
          voiceToggle.checked = voiceEnabled;
          voiceToggle.addEventListener("change", () => {
            voiceEnabled = voiceToggle.checked;
            try {
              localStorage.setItem(voiceStorageKey, voiceEnabled ? "1" : "0");
            } catch {}
          });
        }
        if ("speechSynthesis" in window) {
          const refreshVoices = () => {
            try {
              availableVoices = window.speechSynthesis.getVoices();
            } catch {
              availableVoices = [];
            }
          };
          refreshVoices();
          window.speechSynthesis.addEventListener("voiceschanged", refreshVoices);
        }

        function pad2(value) {
          return String(value).padStart(2, "0");
        }

        function formatTimeLeft(ms) {
          const clamped = Math.max(0, ms);
          const totalSec = Math.floor(clamped / 1000);
          const minutes = Math.floor(totalSec / 60);
          const seconds = totalSec % 60;
          const millis = Math.floor(clamped % 1000);
          return `${pad2(minutes)}:${pad2(seconds)}.${String(millis).padStart(3, "0")}`;
        }

        function resize() {
          const rect = canvas.getBoundingClientRect();
          viewWidth = rect.width;
          viewHeight = rect.height;
          const dpr = window.devicePixelRatio || 1;
          canvas.width = rect.width * dpr;
          canvas.height = rect.height * dpr;
          ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        }

        function lerp(a, b, t) {
          return a + (b - a) * t;
        }

        function line(ax, ay, bx, by) {
          ctx.beginPath();
          ctx.moveTo(ax, ay);
          ctx.lineTo(bx, by);
          ctx.stroke();
        }

        function roundedRectPath(x, y, w, h, r) {
          const radius = Math.min(r, w / 2, h / 2);
          ctx.beginPath();
          ctx.moveTo(x + radius, y);
          ctx.arcTo(x + w, y, x + w, y + h, radius);
          ctx.arcTo(x + w, y + h, x, y + h, radius);
          ctx.arcTo(x, y + h, x, y, radius);
          ctx.arcTo(x, y, x + w, y, radius);
          ctx.closePath();
        }

        function drawCanvasBackdrop() {
          const w = viewWidth;
          const h = viewHeight;
          if (!w || !h) {
            return;
          }
          const bg = ctx.createLinearGradient(0, 0, w, h);
          bg.addColorStop(0, "#fbf7f0");
          bg.addColorStop(1, "#efe4d4");
          ctx.fillStyle = bg;
          ctx.fillRect(0, 0, w, h);

          const glowA = ctx.createRadialGradient(
            w * 0.8,
            h * 0.15,
            10,
            w * 0.8,
            h * 0.15,
            Math.max(w, h) * 0.7
          );
          glowA.addColorStop(0, "rgba(47, 111, 109, 0.18)");
          glowA.addColorStop(1, "rgba(47, 111, 109, 0)");
          ctx.fillStyle = glowA;
          ctx.fillRect(0, 0, w, h);

          const glowB = ctx.createRadialGradient(
            w * 0.2,
            h * 0.9,
            10,
            w * 0.2,
            h * 0.9,
            Math.max(w, h) * 0.75
          );
          glowB.addColorStop(0, "rgba(194, 74, 58, 0.16)");
          glowB.addColorStop(1, "rgba(194, 74, 58, 0)");
          ctx.fillStyle = glowB;
          ctx.fillRect(0, 0, w, h);

          const spacing = Math.max(36, Math.floor(Math.min(w, h) * 0.12));
          ctx.save();
          ctx.strokeStyle = palette.grid;
          ctx.lineWidth = 1;
          for (let x = spacing; x < w; x += spacing) {
            ctx.beginPath();
            ctx.moveTo(x, 0);
            ctx.lineTo(x, h);
            ctx.stroke();
          }
          for (let y = spacing; y < h; y += spacing) {
            ctx.beginPath();
            ctx.moveTo(0, y);
            ctx.lineTo(w, y);
            ctx.stroke();
          }
          ctx.restore();
        }

        function drawPanel(x, y, w, h, r, softShadow = true) {
          ctx.save();
          ctx.fillStyle = palette.paper;
          ctx.shadowColor = softShadow ? "rgba(29, 28, 26, 0.18)" : "transparent";
          ctx.shadowBlur = softShadow ? 14 : 0;
          ctx.shadowOffsetY = softShadow ? 8 : 0;
          roundedRectPath(x, y, w, h, r);
          ctx.fill();
          ctx.shadowColor = "transparent";
          ctx.strokeStyle = "rgba(29, 28, 26, 0.14)";
          ctx.lineWidth = 1.5;
          ctx.stroke();
          ctx.restore();
        }

        function drawProgressBar(x, y, w, h, percent, gradient) {
          const clamped = Math.max(0, Math.min(100, percent));
          const radius = Math.min(12, h / 2);
          drawPanel(x - 4, y - 4, w + 8, h + 8, radius + 4, false);
          ctx.save();
          roundedRectPath(x, y, w, h, radius);
          ctx.clip();
          ctx.fillStyle = palette.paperStrong;
          ctx.fillRect(x, y, w, h);
          if (clamped > 0) {
            ctx.fillStyle = gradient;
            ctx.fillRect(x, y, (w * clamped) / 100, h);
          }
          ctx.restore();
        }

        function drawVerticalProgressBar(x, y, w, h, percent, gradient) {
          const clamped = Math.max(0, Math.min(100, percent));
          const radius = Math.min(12, w / 2);
          drawPanel(x - 4, y - 4, w + 8, h + 8, radius + 4, false);
          ctx.save();
          roundedRectPath(x, y, w, h, radius);
          ctx.clip();
          ctx.fillStyle = palette.paperStrong;
          ctx.fillRect(x, y, w, h);
          if (clamped > 0) {
            ctx.fillStyle = gradient;
            const fillHeight = (h * clamped) / 100;
            ctx.fillRect(x, y + h - fillHeight, w, fillHeight);
          }
          ctx.restore();
        }

        function drawTimeOverlay(text) {
          const w = viewWidth;
          const h = viewHeight;
          if (!w || !h) {
            return;
          }
          const fontSize = Math.max(24, Math.floor(h * 0.075));
          const paddingX = Math.floor(fontSize * 0.5);
          const paddingY = Math.floor(fontSize * 0.35);
          ctx.save();
          ctx.font = `700 ${fontSize}px ${fontMono}`;
          ctx.textAlign = "left";
          ctx.textBaseline = "middle";
          const metrics = ctx.measureText(text);
          const boxW = metrics.width + paddingX * 2;
          const boxH = fontSize + paddingY * 2;
          const x = w - boxW - paddingX;
          const y = paddingY;
          drawPanel(x, y, boxW, boxH, Math.min(16, boxH / 2));
          ctx.fillStyle = palette.accent;
          ctx.fillRect(x, y, 4, boxH);
          ctx.fillStyle = palette.ink;
          ctx.fillText(text, x + paddingX, y + boxH / 2);
          ctx.restore();
        }

        function drawCallout(now) {
          if (!calloutText) {
            return;
          }
          if (now > calloutUntil) {
            return;
          }
          const w = viewWidth;
          const h = viewHeight;
          if (!w || !h) {
            return;
          }
          const elapsed = Math.max(0, now - calloutStart);
          const progress = Math.min(1, elapsed / calloutDurationMs);
          const alpha = 1 - progress;
          const scale = 1 + (1 - progress) * 0.06;
          const fontSize = Math.max(26, Math.floor(h * 0.12));

          ctx.save();
          ctx.translate(w / 2, h * 0.46);
          ctx.scale(scale, scale);
          ctx.globalAlpha = alpha;
          ctx.textAlign = "center";
          ctx.textBaseline = "middle";
          ctx.font = `800 ${fontSize}px ${fontSans}`;
          ctx.shadowColor = "rgba(29, 28, 26, 0.2)";
          ctx.shadowBlur = 16;
          ctx.fillStyle = palette.ink;
          ctx.fillText(calloutText, 0, 0);
          ctx.restore();
        }

        function drawProgressOverlay(moveValue, holdValue) {
          const w = viewWidth;
          const h = viewHeight;
          if (!w || !h) {
            return;
          }
          const clampedMove = Math.max(0, Math.min(100, moveValue));
          const clampedHold = Math.max(0, Math.min(100, holdValue));
          const barWidth = Math.max(18, Math.floor(w * 0.03));
          const barHeight = Math.max(160, Math.floor(h * 0.6));
          const barX = Math.max(16, Math.floor(w * 0.04));
          const barY = Math.floor((h - barHeight) * 0.5);
          const gap = Math.max(24, Math.floor(barWidth * 1.8));
          const holdBarX = barX + barWidth + gap;
          const moveFillHeight = Math.floor((barHeight * clampedMove) / 100);
          const holdFillHeight = Math.floor((barHeight * clampedHold) / 100);

          ctx.save();
          const moveGradient = ctx.createLinearGradient(0, barY + barHeight, 0, barY);
          moveGradient.addColorStop(0, palette.accent);
          moveGradient.addColorStop(1, "rgba(194, 74, 58, 0.2)");
          const holdGradient = ctx.createLinearGradient(0, barY + barHeight, 0, barY);
          holdGradient.addColorStop(0, palette.accent2);
          holdGradient.addColorStop(1, "rgba(47, 111, 109, 0.2)");
          drawVerticalProgressBar(barX, barY, barWidth, barHeight, clampedMove, moveGradient);
          drawVerticalProgressBar(holdBarX, barY, barWidth, barHeight, clampedHold, holdGradient);

          const fontSize = Math.max(14, Math.floor(h * 0.055));
          ctx.font = `700 ${fontSize}px ${fontSans}`;
          ctx.fillStyle = palette.ink;
          ctx.textAlign = "left";
          ctx.textBaseline = "top";
          ctx.fillText(clampedMove.toFixed(0), barX, barY - fontSize - 10);
          ctx.fillText(clampedHold.toFixed(0), holdBarX, barY - fontSize - 10);
          ctx.restore();
        }

        function drawHorizontalProgress(value, y, label) {
          const w = viewWidth;
          const h = viewHeight;
          if (!w || !h) {
            return;
          }
          const clamped = Math.max(0, Math.min(100, value));
          const barWidth = Math.max(220, Math.floor(w * 0.55));
          const barHeight = Math.max(14, Math.floor(h * 0.03));
          const x = Math.floor((w - barWidth) * 0.5);

          const gradient = ctx.createLinearGradient(x, y, x + barWidth, y);
          gradient.addColorStop(0, palette.accent);
          gradient.addColorStop(1, palette.accent2);
          drawProgressBar(x, y, barWidth, barHeight, clamped, gradient);

          const fontSize = Math.max(13, Math.floor(h * 0.035));
          const textX = x + barWidth + Math.max(12, Math.floor(w * 0.02));
          const textY = y + barHeight / 2;
          ctx.font = `700 ${fontSize}px ${fontSans}`;
          ctx.fillStyle = palette.ink;
          ctx.textAlign = "left";
          ctx.textBaseline = "middle";
          ctx.fillText(`${label} ${clamped.toFixed(1)}%`, textX, textY);
        }

        function drawBottomProgressBars() {
          const h = viewHeight;
          if (!h) {
            return;
          }
          const barHeight = Math.max(14, Math.floor(h * 0.03));
          const gap = Math.max(10, Math.floor(barHeight * 1.2));
          const overallY = Math.floor(h * 0.9);
          const setY = overallY - barHeight - gap;
          drawHorizontalProgress(lastSetProgress, setY, "SET");
          drawHorizontalProgress(lastOverallProgress, overallY, "TOTAL");
        }

        function drawRestProgress(value) {
          if (!restActive) {
            return;
          }
          const w = viewWidth;
          const h = viewHeight;
          if (!w || !h) {
            return;
          }
          const clamped = Math.max(0, Math.min(100, value));
          const barWidth = Math.max(140, Math.floor(w * 0.28));
          const barHeight = Math.max(12, Math.floor(h * 0.025));
          const x = Math.floor(w * 0.62);
          const y = Math.floor(h * 0.5);

          const gradient = ctx.createLinearGradient(x, y, x + barWidth, y);
          gradient.addColorStop(0, palette.accent2);
          gradient.addColorStop(1, "rgba(47, 111, 109, 0.3)");
          drawProgressBar(x, y, barWidth, barHeight, clamped, gradient);

          const fontSize = Math.max(13, Math.floor(h * 0.035));
          ctx.font = `700 ${fontSize}px ${fontSans}`;
          ctx.fillStyle = palette.ink;
          ctx.textAlign = "left";
          ctx.textBaseline = "bottom";
          ctx.fillText(`REST ${clamped.toFixed(0)}%`, x, y - 6);
        }

        function drawCountdown(value) {
          const w = viewWidth;
          const h = viewHeight;
          if (!w || !h) {
            return;
          }
          ctx.clearRect(0, 0, w, h);
          drawCanvasBackdrop();
          ctx.fillStyle = palette.ink;
          ctx.textAlign = "center";
          ctx.textBaseline = "middle";
          const fontSize = Math.max(48, Math.floor(h * 0.5));
          ctx.font = `700 ${fontSize}px ${fontSans}`;
          ctx.shadowColor = "rgba(29, 28, 26, 0.2)";
          ctx.shadowBlur = 14;
          ctx.fillText(String(value), w / 2, h / 2);
          ctx.shadowColor = "transparent";
          drawTimeOverlay(lastTimeLeft);
          drawProgressOverlay(lastMoveProgress, lastHoldProgress);
          drawBottomProgressBars();
          drawRestProgress(lastRestProgress);
        }

        function drawStartPrompt() {
          const w = viewWidth;
          const h = viewHeight;
          if (!w || !h) {
            return;
          }
          ctx.clearRect(0, 0, w, h);
          drawCanvasBackdrop();
          ctx.fillStyle = palette.ink;
          ctx.textAlign = "center";
          ctx.textBaseline = "middle";
          const fontSize = Math.max(22, Math.floor(h * 0.12));
          const lineHeight = Math.floor(fontSize * 1.15);
          ctx.font = `700 ${fontSize}px ${fontSans}`;
          const lines = isTouch
            ? ["TAP", "TO", "SQUAT"]
            : ["PRESS", "ENTER", "TO", "SQUAT"];
          const totalHeight = lineHeight * (lines.length - 1);
          let y = h / 2 - totalHeight / 2;
          for (const line of lines) {
            ctx.fillText(line, w / 2, y);
            y += lineHeight;
          }
          drawTimeOverlay(lastTimeLeft);
          drawProgressOverlay(lastMoveProgress, lastHoldProgress);
          drawBottomProgressBars();
          drawRestProgress(lastRestProgress);
        }

        function drawFigure(progress) {
          const w = viewWidth;
          const h = viewHeight;
          if (!w || !h) {
            return;
          }
          ctx.clearRect(0, 0, w, h);
          drawCanvasBackdrop();
          ctx.lineCap = "round";
          ctx.lineJoin = "round";

          const ground = h * 0.86;
          const scale = Math.min(w, h) / 340;
          const headR = 12 * scale;
          const torso = 78 * scale;
          const thigh = 80 * scale;
          const shin = 80 * scale;
          const shoulder = 34 * scale;
          const hip = 30 * scale;

          const depth = Math.max(0, Math.min(1, progress));
          const fatigue = Math.max(0, Math.min(1, lastOverallProgress / 100));
          const restFactor = restActive ? 1 - lastRestProgress / 100 : 1;
          const swing = swingStart + (swingStop - swingStart) * (fatigue * fatigue);
          const tremorAmp = scale * swing;
          const base = (Math.PI * 2 * freq * tremorTime) / 1000;
          const tremor =
            Math.sin(base) * tremorAmp * restFactor +
            Math.sin(base * 2.4) * tremorAmp * 0.4 * restFactor;
          const hipTop = ground - (thigh + shin);
          const hipBottom = ground - shin + 6 * scale;
          const hipY = lerp(hipTop, hipBottom, depth);
          const hipX = w * 0.5 + tremor;
          const shoulderX = hipX - depth * 26 * scale + tremor * 0.3;
          const shoulderY = hipY - torso + depth * 12 * scale;

          const footY = ground;
          const footSpread = 36 * scale;
          const ankleLX = w * 0.5 - footSpread;
          const ankleRX = w * 0.5 + footSpread;
          const hipLX = hipX - hip * 0.4;
          const hipRX = hipX + hip * 0.4;

          function kneeFromHip(hipX, hipY, ankleX, ankleY, outwardLeft) {
            const dx = ankleX - hipX;
            const dy = ankleY - hipY;
            const dist = Math.max(0.001, Math.hypot(dx, dy));
            const maxReach = thigh + shin - 0.001;
            const minReach = Math.abs(thigh - shin) + 0.001;
            const d = Math.max(minReach, Math.min(maxReach, dist));
            const a = (thigh * thigh - shin * shin + d * d) / (2 * d);
            const h = Math.sqrt(Math.max(thigh * thigh - a * a, 0));
            const ux = dx / d;
            const uy = dy / d;
            const px = hipX + a * ux;
            const py = hipY + a * uy;
            const perpX = -uy;
            const perpY = ux;
            const k1 = { x: px + h * perpX, y: py + h * perpY };
            const k2 = { x: px - h * perpX, y: py - h * perpY };
            if (outwardLeft) {
              return k1.x < k2.x ? k1 : k2;
            }
            return k1.x > k2.x ? k1 : k2;
          }

          const kneeL = kneeFromHip(hipLX, hipY, ankleLX, footY, true);
          const kneeR = kneeFromHip(hipRX, hipY, ankleRX, footY, false);

          ctx.save();
          ctx.strokeStyle = "rgba(29, 28, 26, 0.18)";
          ctx.lineWidth = Math.max(2, 2.2 * scale);
          ctx.setLineDash([10 * scale, 12 * scale]);
          line(0, ground, w, ground);
          ctx.restore();

          const figureGradient = ctx.createLinearGradient(
            0,
            shoulderY - headR * 2,
            0,
            ground
          );
          figureGradient.addColorStop(0, palette.ink);
          figureGradient.addColorStop(1, palette.accent2);
          ctx.save();
          ctx.strokeStyle = figureGradient;
          ctx.lineWidth = Math.max(2.2, 2.8 * scale);
          ctx.shadowColor = "rgba(29, 28, 26, 0.2)";
          ctx.shadowBlur = 8 * scale;
          ctx.shadowOffsetY = 3 * scale;
          ctx.setLineDash([]);
          line(0, ground, w, ground);
          line(shoulderX, shoulderY, hipX, hipY);

          ctx.beginPath();
          ctx.arc(shoulderX, shoulderY - headR * 1.6, headR, 0, Math.PI * 2);
          const headGradient = ctx.createRadialGradient(
            shoulderX - headR * 0.3,
            shoulderY - headR * 1.9,
            headR * 0.2,
            shoulderX,
            shoulderY - headR * 1.6,
            headR * 1.2
          );
          headGradient.addColorStop(0, "rgba(255, 255, 255, 0.95)");
          headGradient.addColorStop(1, "rgba(194, 74, 58, 0.7)");
          ctx.fillStyle = headGradient;
          ctx.fill();
          ctx.stroke();

          const armDrop = 26 * scale + depth * 10 * scale;
          line(shoulderX, shoulderY + 6 * scale, shoulderX - shoulder * 0.5, shoulderY + armDrop);
          line(shoulderX, shoulderY + 6 * scale, shoulderX + shoulder * 0.5, shoulderY + armDrop);

          line(hipX - hip * 0.5, hipY, hipX + hip * 0.5, hipY);
          line(hipLX, hipY, kneeL.x, kneeL.y);
          line(kneeL.x, kneeL.y, ankleLX, footY);
          line(hipRX, hipY, kneeR.x, kneeR.y);
          line(kneeR.x, kneeR.y, ankleRX, footY);

          const foot = 14 * scale;
          line(ankleLX - foot, footY, ankleLX + foot, footY);
          line(ankleRX - foot, footY, ankleRX + foot, footY);
          ctx.restore();
          drawCallout(tremorTime);
          drawTimeOverlay(lastTimeLeft);
          drawProgressOverlay(lastMoveProgress, lastHoldProgress);
          drawBottomProgressBars();
          drawRestProgress(lastRestProgress);
        }

        function update() {
          if (stopped) {
            return;
          }
          const now = performance.now();
          if (!started) {
            currentProgress = 0;
            lastMoveProgress = 0;
            lastHoldProgress = 0;
            lastOverallProgress = 0;
            lastSetProgress = 0;
            lastRestProgress = 0;
            restActive = false;
            lastPhase = "";
            calloutText = "";
            lastCountdownSpoken = null;
            line1.textContent = `Slow Squat  Set: 1/${sets}  Rep: 1/${count}`;
            line2.textContent = `Phase: DOWN  Tempo: down ${down.toFixed(
              1
            )}s / hold ${hold.toFixed(1)}s / up ${up.toFixed(1)}s`;
            lastTimeLeft = formatTimeLeft(overallTotal * 1000);
            line4.textContent = `Time left: ${lastTimeLeft}`;
            if (!countdownStarted) {
              line5.textContent = isTouch ? "Status: WAITING (TAP)" : "Status: WAITING (ENTER)";
              drawStartPrompt();
              requestAnimationFrame(update);
              return;
            }
            const elapsedCountdown = now - countdownStart;
            const remainingCountdown = Math.max(
              1,
              countdownSeconds - Math.floor(elapsedCountdown / 1000)
            );
            if (lastCountdownSpoken !== remainingCountdown) {
              speakCountdown(remainingCountdown);
              lastCountdownSpoken = remainingCountdown;
            }
            line5.textContent = `Status: COUNTDOWN ${remainingCountdown}`;
            drawCountdown(remainingCountdown);

            if (elapsedCountdown >= countdownSeconds * 1000) {
              started = true;
              animationStart = performance.now();
              paused = false;
              pauseStarted = null;
              pausedTotal = 0;
            } else {
              requestAnimationFrame(update);
              return;
            }
          }

          const effectiveNow = paused && pauseStarted ? pauseStarted : now;
          tremorTime = effectiveNow;
          const elapsed = Math.max(0, effectiveNow - animationStart - pausedTotal);
          const overallMs = overallTotal * 1000;
          const done = elapsed >= overallMs;
          let phase = "DOWN";
          let depth = 0;
          let moveProgress = lastMoveProgress;
          let holdProgress = lastHoldProgress;
          let restRemainingMs = 0;
          let setIndex = 0;
          let withinSetMs = 0;
          let isRest = false;
          let completed = 0;

          if (!done) {
            const setMs = total * 1000;
            const restMs = interval * 1000;
            const cycleMs = setMs + restMs;
            setIndex = Math.floor(elapsed / cycleMs);
            if (setIndex >= sets) {
              setIndex = sets - 1;
            }
            const withinCycle = elapsed - setIndex * cycleMs;
            if (setIndex < sets - 1 && withinCycle >= setMs) {
              isRest = true;
              restRemainingMs = Math.max(0, restMs - (withinCycle - setMs));
            } else {
              withinSetMs = withinCycle;
            }
          }

          if (done) {
            phase = "UP";
            depth = 0;
            moveProgress = 100;
            completed = count;
            lastSetProgress = 100;
            restActive = false;
            lastRestProgress = 100;
          } else if (isRest) {
            phase = "REST";
            depth = 0;
            moveProgress = 0;
            holdProgress = 0;
            completed = 0;
            lastSetProgress = 100;
            restActive = true;
            if (interval > 0) {
              lastRestProgress = Math.max(
                0,
                Math.min(100, ((interval * 1000 - restRemainingMs) / (interval * 1000)) * 100)
              );
            } else {
              lastRestProgress = 100;
            }
          } else {
            restActive = false;
            lastRestProgress = 0;
            const withinSetSec = withinSetMs / 1000;
            completed = Math.min(Math.floor(withinSetSec / repDuration), count);
            const within = withinSetSec - completed * repDuration;
            if (within < down) {
              phase = "DOWN";
              const t = down > 0 ? within / down : 1;
              depth = t;
              moveProgress = t * 100;
            } else if (within < down + hold) {
              phase = "HOLD";
              depth = 1;
              const t = hold > 0 ? (within - down) / hold : 1;
              holdProgress = t * 100;
              moveProgress = 100;
            } else {
              phase = "UP";
              const t = up > 0 ? (within - down - hold) / up : 1;
              depth = 1 - t;
              moveProgress = t * 100;
            }
            lastSetProgress = Math.max(0, Math.min(100, (withinSetSec / total) * 100));
          }

          const clamped = Math.max(0, Math.min(1, depth));
          currentProgress = clamped;
          lastMoveProgress = Math.max(0, Math.min(100, moveProgress));
          if (phase === "HOLD") {
            lastHoldProgress = Math.max(0, Math.min(100, holdProgress));
          }
          const remaining = Math.max(0, overallMs - elapsed);
          lastOverallProgress = Math.max(0, Math.min(100, (elapsed / overallMs) * 100));
          lastTimeLeft = formatTimeLeft(remaining);
          const displaySet = done
            ? sets
            : isRest && setIndex < sets - 1
              ? setIndex + 2
              : setIndex + 1;
          const current = isRest ? 0 : Math.min(completed + 1, count);

          if (!done && !isRest) {
            if (phase !== lastPhase) {
              triggerCallout(phase, effectiveNow);
              lastPhase = phase;
            }
          } else {
            lastPhase = phase;
          }

          line1.textContent = `Slow Squat  Set: ${displaySet}/${sets}  Rep: ${done ? count : current}/${count}`;
          line2.textContent = `Phase: ${phase}  Tempo: down ${down.toFixed(
            1
          )}s / hold ${hold.toFixed(1)}s / up ${up.toFixed(1)}s`;
          line4.textContent = `Time left: ${lastTimeLeft}`;
          if (paused) {
            line5.textContent = "Status: PAUSED";
          } else if (done) {
            line5.textContent = "Status: COMPLETE";
          } else if (isRest) {
            line5.textContent = `Status: REST ${formatTimeLeft(restRemainingMs)}`;
          } else {
            line5.textContent = "Status: RUNNING";
          }

          drawFigure(clamped);

          if (!done && !stopped) {
            requestAnimationFrame(update);
          }
        }

        function speakText(text) {
          if (!voiceEnabled) {
            return;
          }
          if (!speechReady) {
            return;
          }
          if (!("speechSynthesis" in window)) {
            return;
          }
          try {
            window.speechSynthesis.cancel();
            window.speechSynthesis.resume();
            const utter = new SpeechSynthesisUtterance(text);
            const preferred = availableVoices.find((voice) =>
              voice.lang.toLowerCase().startsWith("en")
            );
            if (preferred) {
              utter.voice = preferred;
            }
            utter.rate = 1;
            utter.pitch = 1;
            utter.volume = 0.9;
            window.speechSynthesis.speak(utter);
          } catch {}
        }

        function speakPhase(text) {
          speakText(text);
        }

        function speakCountdown(value) {
          speakText(String(value));
        }

        function unlockSpeech() {
          if (!voiceEnabled || speechReady) {
            return;
          }
          if (!("speechSynthesis" in window)) {
            return;
          }
          speechReady = true;
          try {
            const utter = new SpeechSynthesisUtterance(" ");
            utter.volume = 0;
            utter.rate = 1;
            utter.pitch = 1;
            window.speechSynthesis.speak(utter);
          } catch {
            speechReady = true;
          }
        }

        function triggerCallout(phase, now) {
          let text = "";
          if (phase === "DOWN") {
            text = "DOWN!";
          } else if (phase === "HOLD") {
            text = "HOLD!";
          } else if (phase === "UP") {
            text = "UP!";
          }
          if (!text) {
            return;
          }
          calloutText = text;
          calloutStart = now;
          calloutUntil = now + calloutDurationMs;
          speakPhase(phase);
        }

        function togglePause() {
          if (stopped || !started) {
            return;
          }
          if (paused) {
            if (pauseStarted !== null) {
              pausedTotal += performance.now() - pauseStarted;
            }
            paused = false;
            pauseStarted = null;
            requestAnimationFrame(update);
          } else {
            paused = true;
            pauseStarted = performance.now();
          }
        }

        function stop() {
          if (stopped) {
            return;
          }
          stopped = true;
          if (pauseStarted !== null) {
            pausedTotal += performance.now() - pauseStarted;
            pauseStarted = null;
          }
          line5.textContent = "Status: STOPPED";
          drawFigure(currentProgress);
        }

        window.addEventListener("resize", () => {
          resize();
          if (!started) {
            if (countdownStarted) {
              drawCountdown(Math.max(1, countdownSeconds));
            } else {
              drawStartPrompt();
            }
          } else {
            drawFigure(currentProgress);
          }
        });

        function startCountdown() {
          if (started || countdownStarted) {
            return;
          }
          unlockSpeech();
          countdownStarted = true;
          countdownStart = performance.now();
          requestAnimationFrame(update);
        }

        function skipCountdown() {
          if (started) {
            return;
          }
          unlockSpeech();
          countdownStarted = true;
          started = true;
          animationStart = performance.now();
          paused = false;
          pauseStarted = null;
          pausedTotal = 0;
          requestAnimationFrame(update);
        }

        window.addEventListener("keydown", (event) => {
          if (event.code === "Enter") {
            if (!countdownStarted) {
              startCountdown();
            } else if (!started) {
              skipCountdown();
            }
            return;
          }
          if (event.code === "Space") {
            event.preventDefault();
            togglePause();
            return;
          }
          if (event.code === "Escape") {
            stop();
            return;
          }
          if ((event.ctrlKey || event.metaKey) && (event.key === "c" || event.key === "C")) {
            stop();
          }
        });

        canvas.addEventListener("pointerdown", (event) => {
          if (!isTouch) {
            return;
          }
          event.preventDefault();
          if (!started) {
            if (!countdownStarted) {
              startCountdown();
            } else {
              skipCountdown();
            }
            return;
          }
          togglePause();
        });

        canvas.addEventListener(
          "touchstart",
          (event) => {
            if (!isTouch || supportsPointer) {
              return;
            }
            event.preventDefault();
            if (!started) {
              if (!countdownStarted) {
                startCountdown();
              } else {
                skipCountdown();
              }
              return;
            }
            togglePause();
          },
          { passive: false }
        );

        resize();
        requestAnimationFrame(update);
      })();
    </script>
  </body>
</html>
"##;

/* trait  ************************************************************************************************/

/* enum  *************************************************************************************************/

#[derive(Subcommand, Debug)]
enum Commands {
  Squat(SquatArgs),
  SquatWeb(SquatWebArgs),
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

#[derive(Args, Debug)]
struct SquatWebArgs {
  #[arg(long, default_value_t = 150, value_parser = clap::value_parser!(u64).range(1..))]
  duration: u64,
  #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..))]
  count: u32,
  #[arg(long = "sets", alias = "set", default_value_t = 2, value_parser = clap::value_parser!(u32).range(1..))]
  sets: u32,
  #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u64).range(0..))]
  interval: u64,
  #[arg(long, default_value_t = 0.4, value_parser = clap::value_parser!(f64))]
  swing_start: f64,
  #[arg(long, default_value_t = 3.4, value_parser = clap::value_parser!(f64))]
  swing_stop: f64,
  #[arg(long, default_value_t = 10.0, value_parser = clap::value_parser!(f64))]
  freq: f64,
  #[arg(long, default_value = "127.0.0.1:12002")]
  addr: String,
}

struct FrameState<'a> {
  current: u32,
  total: u32,
  phase: &'a str,
  down_secs: f64,
  hold_secs: f64,
  up_secs: f64,
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

fn squat_web_html(
  duration: u64,
  count: u32,
  sets: u32,
  interval: u64,
  swing_start: f64,
  swing_stop: f64,
  freq: f64,
) -> String {
  SQUAT_WEB_HTML
    .replace("__DURATION__", &duration.to_string())
    .replace("__COUNT__", &count.to_string())
    .replace("__SETS__", &sets.to_string())
    .replace("__INTERVAL__", &interval.to_string())
    .replace("__VERSION__", APP_VERSION)
    .replace("__SWING_START__", &format!("{:.3}", swing_start))
    .replace("__SWING_STOP__", &format!("{:.3}", swing_stop))
    .replace("__FREQ__", &format!("{:.3}", freq))
    .replace("__HOLD__", &format!("{:.1}", HOLD_SECS))
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
    "Phase: {}  Tempo: down {:.1}s / hold {:.1}s / up {:.1}s\r\n",
    state.phase, state.down_secs, state.hold_secs, state.up_secs
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
  if rep_duration <= HOLD_SECS {
    return Err(color_eyre::eyre::eyre!(
      "duration/count must be greater than {:.1}s to allow a {:.1}s hold",
      HOLD_SECS,
      HOLD_SECS
    ));
  }
  let down_duration = (rep_duration - HOLD_SECS) / 2.0;
  let up_duration = down_duration;

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

    let (phase, progress) = if within_rep < down_duration {
      ("DOWN", within_rep / down_duration)
    } else if within_rep < down_duration + HOLD_SECS {
      ("HOLD", 1.0)
    } else {
      (
        "UP",
        1.0 - (within_rep - down_duration - HOLD_SECS) / up_duration,
      )
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
      down_secs: down_duration,
      hold_secs: HOLD_SECS,
      up_secs: up_duration,
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

fn run_squat_web(args: SquatWebArgs) -> Result<()> {
  let rep_duration = args.duration as f64 / args.count as f64;
  if rep_duration <= HOLD_SECS {
    return Err(color_eyre::eyre::eyre!(
      "duration/count must be greater than {:.1}s to allow a {:.1}s hold",
      HOLD_SECS,
      HOLD_SECS
    ));
  }
  if args.swing_start.is_sign_negative()
    || args.swing_stop.is_sign_negative()
    || args.freq.is_sign_negative()
  {
    return Err(color_eyre::eyre::eyre!(
      "swing-start, swing-stop, and freq must be >= 0"
    ));
  }
  let exit_flag = Arc::new(AtomicBool::new(false));
  let exit_flag_clone = exit_flag.clone();
  ctrlc::set_handler(move || {
    exit_flag_clone.store(true, Ordering::SeqCst);
  })?;

  let server = Server::http(&args.addr).map_err(|err| color_eyre::eyre::eyre!(err))?;
  let html = squat_web_html(
    args.duration,
    args.count,
    args.sets,
    args.interval,
    args.swing_start,
    args.swing_stop,
    args.freq,
  );
  let content_type = Header::from_bytes("Content-Type", "text/html; charset=utf-8")
    .map_err(|_| color_eyre::eyre::eyre!("invalid content-type header"))?;

  while !exit_flag.load(Ordering::SeqCst) {
    match server.recv_timeout(Duration::from_millis(200)) {
      Ok(Some(request)) => {
        let response = Response::from_string(html.clone()).with_header(content_type.clone());
        let _ = request.respond(response);
      }
      Ok(None) => {}
      Err(err) => return Err(err.into()),
    }
  }

  Ok(())
}

fn main() -> Result<()> {
  color_eyre::install()?;
  init_tracing()?;

  let cli = Cli::parse();

  match cli.command {
    Commands::Squat(args) => run_squat(args),
    Commands::SquatWeb(args) => run_squat_web(args),
  }
}

/* async fn  *********************************************************************************************/

/* test for pri ******************************************************************************************/

/* test for pub ******************************************************************************************/
