//! 音频核心：ffmpeg 解码 → 16bit WAV 加载 → cpal 输出流 + 频谱分析。
//!
//! 设计：整曲解码为内存中的立体声 i16 样本（经 ffmpeg 统一转码/重采样/降混），
//! cpal 回调从中按播放位置取数写入声卡；UI 通过同一份样本做频谱可视化。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub const AUDIO_EXTS: &[&str] = &[
    "mp3", "flac", "wav", "m4a", "aac", "aiff", "aif", "ogg", "oga", "opus",
    "mp4", "wma", "ape", "mka",
];

pub fn is_audio_path(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// ffmpeg 可执行文件候选路径（非 Windows 常见安装位置，兜底用）
const FFMPEG_CANDIDATES: &[&str] = &[
    "/opt/homebrew/bin/ffmpeg",
    "/usr/local/bin/ffmpeg",
    "/opt/local/bin/ffmpeg",
    "/usr/bin/ffmpeg",
];

fn ffmpeg_exe_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}

/// 定位 ffmpeg，优先级：
/// 1. 环境变量 TMUS_FFMPEG
/// 2. 与 tmus 可执行文件同目录（分发时把 ffmpeg 放一起 → 用户免安装）
/// 3. 常见绝对路径（macOS Homebrew 等）
/// 4. 都不在时返回 None，由调用方回退到 PATH 里的 `ffmpeg`
fn locate_ffmpeg() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TMUS_FFMPEG") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(ffmpeg_exe_name());
            if p.is_file() {
                return Some(p);
            }
        }
    }
    FFMPEG_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.is_file())
}

/// 解码后的整曲数据（立体声 i16，设备采样率）
pub struct PlayData {
    pub samples: Arc<Vec<i16>>,
    pub frames: usize,
}

/// 把任意音频解码为“设备采样率 / 立体声 16bit”内存样本
pub fn load_playable(path: &Path, out_rate: u32) -> Result<PlayData, String> {
    let tmp = std::env::temp_dir().join(format!(
        "tmus-{}-{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));

    decode_via_ffmpeg(path, &tmp, out_rate)?;
    let samples = parse_wav16_stereo(&tmp)?;
    let _ = std::fs::remove_file(&tmp);

    let frames = samples.len() / 2;
    if frames == 0 {
        return Err("解码结果为空".into());
    }
    Ok(PlayData {
        samples: Arc::new(samples),
        frames,
    })
}

fn decode_via_ffmpeg(src: &Path, dst: &Path, out_rate: u32) -> Result<(), String> {
    let ffmpeg = locate_ffmpeg();
    let mut cmd = match &ffmpeg {
        Some(p) => std::process::Command::new(p),
        None => std::process::Command::new("ffmpeg"), // 回退 PATH
    };
    let out = cmd
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-i")
        .arg(src)
        .arg("-vn")
        .arg("-ac")
        .arg("2")
        .arg("-ar")
        .arg(out_rate.to_string())
        .arg("-f")
        .arg("wav")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg(dst)
        .output()
        .map_err(|e| {
            let hint = match &ffmpeg {
                Some(p) => format!("无法启动 ffmpeg({}): {e}", p.display()),
                None => format!(
                    "未找到 ffmpeg: {e}。请把 ffmpeg{} 放在 tmus{} 同目录，\
                     或安装后加入 PATH（macOS: brew install ffmpeg；Windows: winget install ffmpeg）",
                    if cfg!(windows) { ".exe" } else { "" },
                    if cfg!(windows) { ".exe" } else { "" }
                ),
            };
            hint
        })?;

    if !out.status.success() {
        let _ = std::fs::remove_file(dst);
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "ffmpeg 解码失败：{}",
            stderr.trim().lines().last().unwrap_or("未知错误")
        ));
    }
    if !dst.is_file() {
        return Err("ffmpeg 未生成输出文件".into());
    }
    Ok(())
}

/// 解析 16bit 立体声 PCM WAV 到 i16 交错样本（fmt=1 或 WAVE_FORMAT_EXTENSIBLE+PCM）
fn parse_wav16_stereo(path: &Path) -> Result<Vec<i16>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| e.to_string())?;

    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("ffmpeg 输出不是合法 WAV".into());
    }

    let mut audio_format = 0u16;
    let mut channels = 0u16;
    let mut bits = 0u16;
    let mut data_off = 0usize;
    let mut data_size = 0u32;

    let mut off = 12usize;
    while off + 8 <= bytes.len() {
        let id = &bytes[off..off + 4];
        let size =
            u32::from_le_bytes(bytes[off + 4..off + 8].try_into().unwrap()) as usize;
        let body = off + 8;
        if body + size > bytes.len() {
            break;
        }
        match id {
            b"fmt " => {
                if size >= 16 {
                    audio_format = u16::from_le_bytes(bytes[body..body + 2].try_into().unwrap());
                    channels = u16::from_le_bytes(bytes[body + 2..body + 4].try_into().unwrap());
                    bits = u16::from_le_bytes(bytes[body + 14..body + 16].try_into().unwrap());
                    if audio_format == 0xfffe && size >= 40 {
                        let sub =
                            u16::from_le_bytes(bytes[body + 24..body + 26].try_into().unwrap());
                        if sub == 1 {
                            audio_format = 1;
                        }
                    }
                }
            }
            b"data" => {
                data_size = u32::from_le_bytes(bytes[body - 4..body].try_into().unwrap());
                data_off = body;
                break;
            }
            _ => {}
        }
        off = body + size + (size & 1);
    }

    if audio_format != 1 {
        return Err(format!("ffmpeg 输出非 PCM（format={audio_format}）"));
    }
    if channels != 2 || bits != 16 {
        return Err(format!(
            "预期立体声16bit，实际 {channels}ch/{bits}bit（请检查 ffmpeg 参数）"
        ));
    }
    let end = (data_off + data_size as usize).min(bytes.len());
    let bytes_data = &bytes[data_off..end];

    // 内存上限保护（约 90 分钟立体声无损）
    if bytes_data.len() > 480_000_000 {
        return Err("解码后数据过大（>约90分钟），当前 TUI 版本暂不支持".into());
    }

    let mut out = Vec::with_capacity(bytes_data.len() / 2);
    let mut i = 0;
    while i + 1 < bytes_data.len() {
        out.push(i16::from_le_bytes([bytes_data[i], bytes_data[i + 1]]));
        i += 2;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// 播放引擎（cpal）
// ---------------------------------------------------------------------------

/// 跨线程共享的播放状态
pub struct PlayState {
    pub samples: Arc<Vec<i16>>,
    pub frames: usize,
    /// 当前播放到的帧号（立体声帧）
    pub pos: usize,
    pub playing: bool,
    pub track_path: String,
}

pub struct Engine {
    pub state: Arc<Mutex<PlayState>>,
    pub device_rate: u32,
    _stream: cpal::Stream,
}

impl Engine {
    /// 用默认输出设备创建常驻播放引擎
    pub fn new() -> Result<(Self, String), String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "未找到默认音频输出设备".to_string())?;
        let dev_name = device.name().unwrap_or_else(|_| "未知设备".into());
        let default = device
            .default_output_config()
            .map_err(|e| format!("读取音频配置失败: {e}"))?;
        let sample_rate = default.sample_rate().0;
        let channels = default.channels();

        let state = Arc::new(Mutex::new(PlayState {
            samples: Arc::new(Vec::new()),
            frames: 0,
            pos: 0,
            playing: false,
            track_path: String::new(),
        }));

        let stream = match default.sample_format() {
            cpal::SampleFormat::F32 => Self::build::<f32>(&device, &default.config(), channels, state.clone())?,
            cpal::SampleFormat::I16 => Self::build::<i16>(&device, &default.config(), channels, state.clone())?,
            cpal::SampleFormat::I32 => Self::build::<i32>(&device, &default.config(), channels, state.clone())?,
            cpal::SampleFormat::U16 => Self::build::<u16>(&device, &default.config(), channels, state.clone())?,
            other => {
                return Err(format!("不支持的输出采样格式 {other:?}"));
            }
        };
        stream.play().map_err(|e| format!("启动音频流失败: {e}"))?;

        Ok((
            Engine {
                state,
                device_rate: sample_rate,
                _stream: stream,
            },
            dev_name,
        ))
    }

    fn build<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        channels: u16,
        state: Arc<Mutex<PlayState>>,
    ) -> Result<cpal::Stream, String>
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        let err_fn = |e| eprintln!("[audio] 输出流错误: {e}");
        let out_channels = channels as usize;
        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [T], _| {
                    let mut guard = match state.lock() {
                        Ok(g) => g,
                        Err(_) => {
                            for s in data.iter_mut() {
                                *s = T::from_sample(0.0f32);
                            }
                            return;
                        }
                    };
                    for chunk in data.chunks_exact_mut(out_channels) {
                        let playing = guard.playing;
                        let can_play = playing && guard.pos < guard.frames;
                        let (l, r) = if can_play {
                            let s = guard.pos * 2;
                            let smp = &guard.samples;
                            (
                                smp[s] as f32 / 32768.0,
                                smp[s + 1] as f32 / 32768.0,
                            )
                        } else {
                            (0.0, 0.0)
                        };
                        for (c, slot) in chunk.iter_mut().enumerate() {
                            let v = match c {
                                0 => l,
                                1 => r,
                                _ => (l + r) * 0.5,
                            };
                            *slot = T::from_sample(v);
                        }
                        if can_play {
                            guard.pos += 1;
                            if guard.pos >= guard.frames {
                                guard.playing = false; // 播放完毕
                            }
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("创建输出流失败: {e}"))?;
        Ok(stream)
    }

    /// 装载新曲目并开始播放
    pub fn play(&self, data: PlayData, path: String) {
        let mut g = self.state.lock().unwrap();
        g.samples = data.samples;
        g.frames = data.frames;
        g.pos = 0;
        g.playing = true;
        g.track_path = path;
    }

    pub fn toggle(&self) -> bool {
        let mut g = self.state.lock().unwrap();
        if g.frames == 0 {
            return false;
        }
        if !g.playing && g.pos >= g.frames - 1 {
            g.pos = 0; // 播完后再按播放 → 从头开始
        }
        g.playing = !g.playing;
        g.playing
    }

    pub fn stop(&self) {
        let mut g = self.state.lock().unwrap();
        g.playing = false;
        g.pos = 0;
    }

    pub fn seek_rel(&self, delta_sec: f64) {
        let mut g = self.state.lock().unwrap();
        if g.frames == 0 {
            return;
        }
        let rate = self.device_rate as f64;
        let max = (g.frames - 1) as f64;
        let target = g.pos as f64 + delta_sec * rate;
        g.pos = target.clamp(0.0, max) as usize;
        if g.pos >= g.frames - 1 {
            g.playing = false;
        } else if !g.playing {
            g.playing = true; // seek 后恢复播放（简单直观）
        }
    }

    /// UI 快照：(当前曲目路径, 是否播放中, 已播秒数, 总秒数)
    pub fn snapshot(&self) -> (String, bool, f64, f64) {
        let g = self.state.lock().unwrap();
        let dur = if g.frames == 0 {
            0.0
        } else {
            g.frames as f64 / self.device_rate as f64
        };
        (
            g.track_path.clone(),
            g.playing,
            g.pos as f64 / self.device_rate as f64,
            dur,
        )
    }

    /// 供可视化读取：当前整曲样本 + 播放位置
    pub fn viz_frame(&self) -> Option<(Arc<Vec<i16>>, usize)> {
        let g = self.state.lock().ok()?;
        if g.frames == 0 {
            None
        } else {
            Some((g.samples.clone(), g.pos))
        }
    }
}

// ---------------------------------------------------------------------------
// 频谱分析（每列一个 bin 的单点 DFT），返回各频段相对幅度 0..1
// ---------------------------------------------------------------------------

pub struct Spectrum {
    window: usize,
    peak: Vec<f32>,
}

impl Spectrum {
    pub fn new(bands: usize) -> Self {
        Spectrum {
            window: 2048,
            peak: vec![0.0; bands],
        }
    }

    pub fn set_bands(&mut self, n: usize) {
        if n != self.peak.len() {
            self.peak.resize(n, 0.0);
        }
    }

    /// samples 为整曲立体声 i16，pos 为当前帧；返回与 self.peak 等长的 0..1 幅度
    pub fn analyze(&mut self, samples: &[i16], pos: usize, rate: u32) -> Vec<f32> {
        let frames = samples.len() / 2;
        let n = self.window;
        if frames < n {
            return vec![0.0; self.peak.len()];
        }
        let start = {
            let s = pos.saturating_sub(n);
            if s + n <= frames {
                s
            } else {
                frames - n
            }
        };
        let win = &samples[start * 2..(start + n) * 2];

        let mut mono = vec![0.0f64; n];
        for (i, pair) in win.chunks_exact(2).enumerate() {
            mono[i] = (pair[0] as f64 + pair[1] as f64) * 0.5 / 32768.0;
        }

        let bands = self.peak.len();
        let mut mags = vec![0f64; bands];
        let sr = rate as f64;
        let f_min = 40.0f64;
        let f_max = (sr * 0.45).max(f_min + 1.0);
        for b in 0..bands {
            let f = if bands == 1 {
                f_min
            } else {
                f_min * (f_max / f_min).powf(b as f64 / (bands - 1) as f64)
            };
            let k = ((f * n as f64 / sr).round() as usize).clamp(1, n / 2 - 1);
            let (mut re, mut im) = (0.0f64, 0.0f64);
            for (i, &x) in mono.iter().enumerate() {
                let angle = 2.0 * std::f64::consts::PI * k as f64 * i as f64 / n as f64;
                re += x * angle.cos();
                im -= x * angle.sin();
            }
            mags[b] = (re * re + im * im).sqrt() / (n as f64 / 2.0);
        }

        let mut out = Vec::with_capacity(bands);
        for b in 0..bands {
            let m = mags[b] as f32;
            let p = self.peak[b] * 0.985;
            self.peak[b] = if m > p { m * 0.3 + p * 0.7 } else { p };
            let p = self.peak[b].max(1e-5);
            let norm = (m / p).sqrt();
            out.push(norm.clamp(0.0, 1.0));
        }
        out
    }
}
