//! tmus —— 终端 TUI 音乐播放器。
//!
//! 用法：
//!   tmus [起始目录]      # 不带参数 = 当前文件夹
//!
//! 快捷键：
//!   ↑/↓ j/k          移动选择          Enter         进入目录 / 播放选中曲目
//!   Backspace h       返回上级目录      Space         播放/暂停
//!   n / p             下一首 / 上一首   ←/→           快退/快进 5 秒
//!   g / G             列表首 / 末       x             停止
//!   r                 刷新列表
//!   v                 切换可视方式（频谱柱/波形/环形频谱）
//!   c                 循环配色预设      C             自定义颜色（输入 #rrggbb）
//!   q / Esc           退出
//!
//! 可视化偏好保存在 ~/.config/tmus/config.json

mod settings;

use tmus::audio;

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use audio::{Engine, Spectrum};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};

use settings::{ColorPref, Settings, VizKind};

type LoadResult = Result<audio::PlayData, String>;

#[derive(Clone)]
enum Row {
    Parent,
    Dir { name: String, path: PathBuf },
    File { name: String, path: PathBuf, size: u64 },
}

impl Row {
    fn name(&self) -> String {
        match self {
            Row::Parent => "..".into(),
            Row::Dir { name, .. } | Row::File { name, .. } => name.clone(),
        }
    }
    fn path(&self) -> Option<PathBuf> {
        match self {
            Row::Parent => None,
            Row::Dir { path, .. } | Row::File { path, .. } => Some(path.clone()),
        }
    }
}

/// 小型文本输入弹层（用于自定义颜色）
struct Prompt {
    label: String,
    buf: String,
}

struct App {
    cwd: PathBuf,
    rows: Vec<Row>,
    sel: usize,
    engine: Engine,
    spec: Spectrum,
    settings: Settings,
    prompt: Option<Prompt>,
    loading: bool,
    tx: mpsc::Sender<LoadResult>,
    rx: mpsc::Receiver<LoadResult>,
    notice: Option<String>,
    quit: bool,
}

impl App {
    fn new(engine: Engine, settings: Settings) -> Self {
        let (tx, rx) = mpsc::channel();
        App {
            cwd: PathBuf::new(),
            rows: Vec::new(),
            sel: 0,
            engine,
            spec: Spectrum::new(96),
            settings,
            prompt: None,
            loading: false,
            tx,
            rx,
            notice: None,
            quit: false,
        }
    }

    fn refresh(&mut self) {
        self.rows = list_dir(&self.cwd);
        self.notice = None;
        if self.sel >= self.rows.len() {
            self.sel = self.rows.len().saturating_sub(1);
        }
    }

    fn cd(&mut self, dir: &Path) {
        let canon = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        if canon.is_dir() {
            self.cwd = canon;
            self.sel = 0;
            self.refresh();
        }
    }

    fn play_path(&mut self, p: &Path) {
        if self.loading {
            self.notice = Some("正在解码上一首，请稍候…".into());
            return;
        }
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let rate = self.engine.device_rate;
        let tx = self.tx.clone();
        let path = p.to_path_buf();
        self.loading = true;
        self.notice = Some(format!("正在解码: {name}"));
        std::thread::spawn(move || {
            let r = audio::load_playable(&path, rate);
            let _ = tx.send(r);
        });
    }

    fn play_selected(&mut self) {
        enum Target {
            Dir(PathBuf),
            File(PathBuf),
            Up,
        }
        let target = match self.rows.get(self.sel) {
            Some(Row::Dir { path, .. }) => Target::Dir(path.clone()),
            Some(Row::File { path, .. }) => Target::File(path.clone()),
            Some(Row::Parent) => Target::Up,
            None => return,
        };
        match target {
            Target::Dir(p) => self.cd(&p),
            Target::File(p) => self.play_path(&p),
            Target::Up => {
                if let Some(parent) = self.cwd.parent() {
                    let p = parent.to_path_buf();
                    self.cd(&p);
                }
            }
        }
    }

    fn step_play(&mut self, delta: i32) {
        let files: Vec<usize> = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| matches!(r, Row::File { .. }).then_some(i))
            .collect();
        if files.is_empty() {
            return;
        }
        let cur_idx = files.iter().position(|&i| {
            self.rows.get(i).and_then(Row::path)
                == Some(self.engine.snapshot().0.into())
        });
        let base = cur_idx.unwrap_or(0) as i32;
        let n = files.len() as i32;
        let target = files[(((base + delta) % n) + n) as usize % files.len()];
        self.sel = target;
        self.play_selected();
    }

    fn check_loader(&mut self) {
        while let Ok(result) = self.rx.try_recv() {
            self.loading = false;
            match result {
                Ok(data) => {
                    let path = match self.rows.get(self.sel).and_then(Row::path) {
                        Some(p) => p,
                        None => {
                            self.notice = Some("没有可播放的曲目".into());
                            continue;
                        }
                    };
                    self.engine.play(data, path.to_string_lossy().into_owned());
                    self.notice = None;
                }
                Err(e) => {
                    self.notice = Some(format!("解码失败: {e}"));
                }
            }
        }
    }

    // ---------- 可视化设置 ----------

    fn cycle_viz(&mut self) {
        self.settings.viz = self.settings.viz.next();
        settings::save(&self.settings);
        self.notice = Some(format!("可视化方式: {}", self.settings.viz.label()));
    }

    fn cycle_color(&mut self) {
        let next = match self.settings.color {
            ColorPref::Preset(i) => ColorPref::Preset((i + 1) % settings::PRESET_COUNT),
            ColorPref::Custom { .. } => ColorPref::Preset(0),
        };
        self.settings.color = next;
        settings::save(&self.settings);
        self.notice = Some(format!("配色: {}", color_label(self.settings.color)));
    }

    fn open_custom_color(&mut self) {
        self.prompt = Some(Prompt {
            label: "输入自定义颜色 #rrggbb（Enter 确认 · Esc 取消）".into(),
            buf: "#".into(),
        });
    }

    fn prompt_commit(&mut self) {
        let buf = self.prompt.take().map(|p| p.buf).unwrap_or_default();
        match settings::parse_hex(&buf) {
            Some((r, g, b)) => {
                self.settings.color = ColorPref::Custom { r, g, b };
                settings::save(&self.settings);
                self.notice = Some(format!("自定义颜色: {}", color_label(self.settings.color)));
            }
            None => {
                self.notice = Some(format!("无效颜色「{buf}」，应形如 #00ff88"));
            }
        }
    }

    fn prompt_cancel(&mut self) {
        self.prompt = None;
        self.notice = None;
    }

    fn handle_prompt_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.prompt_cancel(),
            KeyCode::Enter => self.prompt_commit(),
            KeyCode::Backspace => {
                if let Some(p) = self.prompt.as_mut() {
                    p.buf.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Some(p) = self.prompt.as_mut() {
                    // 只接受 # 与十六进制字符
                    if c.is_ascii_hexdigit() || (c == '#' && p.buf.is_empty()) {
                        if p.buf.len() < 7 {
                            p.buf.push(c);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, code: KeyCode) {
        if self.prompt.is_some() {
            self.handle_prompt_key(code);
            return;
        }
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.move_sel(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_sel(-1),
            KeyCode::Char('g') | KeyCode::Home => self.sel = 0,
            KeyCode::Char('G') | KeyCode::End => self.sel = self.rows.len().saturating_sub(1),
            KeyCode::Enter => self.play_selected(),
            KeyCode::Char('h') | KeyCode::Backspace => {
                if let Some(parent) = self.cwd.parent() {
                    let p = parent.to_path_buf();
                    self.cd(&p);
                }
            }
            KeyCode::Char(' ') => {
                let (_, _, _, dur) = self.engine.snapshot();
                if dur > 0.0 {
                    self.engine.toggle();
                } else {
                    self.play_selected();
                }
            }
            KeyCode::Char('n') => self.step_play(1),
            KeyCode::Char('p') => self.step_play(-1),
            KeyCode::Char('x') => self.engine.stop(),
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('v') => self.cycle_viz(),
            KeyCode::Char('c') => self.cycle_color(),
            KeyCode::Char('C') => self.open_custom_color(),
            KeyCode::Left => self.engine.seek_rel(-5.0),
            KeyCode::Right => self.engine.seek_rel(5.0),
            _ => {}
        }
    }

    fn move_sel(&mut self, d: i32) {
        let len = self.rows.len() as i32;
        if len == 0 {
            return;
        }
        self.sel = ((self.sel as i32 + d).rem_euclid(len)) as usize;
    }
}

fn list_dir(dir: &Path) -> Vec<Row> {
    let mut dirs: Vec<(String, PathBuf)> = Vec::new();
    let mut files: Vec<(String, PathBuf, u64)> = Vec::new();

    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue; // 隐藏文件
            }
            let p = entry.path();
            if p.is_dir() {
                dirs.push((name, p));
            } else if audio::is_audio_path(&p) {
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                files.push((name, p, size));
            }
        }
    }

    dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

    let mut rows = Vec::new();
    if dir.parent().is_some() {
        rows.push(Row::Parent);
    }
    for (name, p) in dirs {
        rows.push(Row::Dir { name, path: p });
    }
    for (name, p, size) in files {
        rows.push(Row::File { name, path: p, size });
    }
    rows
}

fn fmt_size(n: u64) -> String {
    if n < 1024 {
        return format!("{n} B");
    }
    let units = ["KB", "MB", "GB"];
    let mut v = n as f64 / 1024.0;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    format!("{:.0} {}", v, units[i])
}

fn fmt_time(sec: f64) -> String {
    let s = sec.max(0.0) as u64;
    let m = s / 60;
    let ss = s % 60;
    let h = m / 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m % 60, ss)
    } else {
        format!("{}:{:02}", m, ss)
    }
}

// ---------------- UI 基础 ----------------

const SEL_BG: Color = Color::Rgb(60, 70, 110);
const DIR_C: Color = Color::LightYellow;
const FILE_C: Color = Color::White;
const DIM_C: Color = Color::DarkGray;
const NOW_C: Color = Color::LightGreen;
const PROG_C: Color = Color::LightCyan;

/// 内置配色预设（RGB 三元组，从暗到亮）
const STOPS: [&[[u8; 3]]; settings::PRESET_COUNT] = [
    // 0 光谱
    &[
        [34, 211, 238],
        [45, 212, 191],
        [74, 222, 128],
        [163, 230, 53],
        [253, 224, 71],
        [251, 146, 60],
        [251, 113, 133],
        [253, 186, 200],
    ],
    // 1 霓虹
    &[
        [96, 165, 250],
        [129, 140, 248],
        [168, 85, 247],
        [192, 38, 211],
        [217, 70, 239],
        [244, 114, 182],
        [255, 121, 221],
    ],
    // 2 森绿
    &[
        [20, 83, 45],
        [22, 101, 52],
        [22, 163, 74],
        [34, 197, 94],
        [74, 222, 128],
        [134, 239, 172],
        [187, 247, 208],
    ],
    // 3 火焰
    &[
        [254, 240, 138],
        [253, 224, 71],
        [250, 204, 21],
        [249, 115, 22],
        [234, 88, 12],
        [194, 65, 12],
    ],
    // 4 冰蓝
    &[
        [7, 89, 133],
        [3, 105, 161],
        [14, 165, 233],
        [56, 189, 248],
        [125, 211, 252],
        [186, 230, 253],
        [224, 242, 254],
    ],
    // 5 云白
    &[
        [100, 116, 139],
        [148, 163, 184],
        [203, 213, 225],
        [226, 232, 240],
        [241, 245, 249],
        [255, 255, 255],
    ],
];

/// 自定义颜色 → 从暗到亮的渐变
fn custom_ramp(r: u8, g: u8, b: u8, steps: usize) -> Vec<[u8; 3]> {
    let denom = steps.saturating_sub(1).max(1) as f32;
    (0..steps)
        .map(|k| {
            let t = 0.22 + 0.78 * k as f32 / denom;
            [
                ((r as f32) * t).round().clamp(0.0, 255.0) as u8,
                ((g as f32) * t).round().clamp(0.0, 255.0) as u8,
                ((b as f32) * t).round().clamp(0.0, 255.0) as u8,
            ]
        })
        .collect()
}

/// 生成 n 段渐变色
fn palette_for(cs: ColorPref, n: usize) -> Vec<Color> {
    let raw: Vec<[u8; 3]> = match cs {
        ColorPref::Preset(i) => STOPS[i % settings::PRESET_COUNT].to_vec(),
        ColorPref::Custom { r, g, b } => custom_ramp(r, g, b, 8),
    };
    if n == 0 || raw.is_empty() {
        return vec![Color::Rgb(34, 211, 238)];
    }
    let last = raw.len() - 1;
    (0..n)
        .map(|i| {
            let idx = if n == 1 { 0 } else { i * last / (n - 1) };
            let c = raw[idx];
            Color::Rgb(c[0], c[1], c[2])
        })
        .collect()
}

/// 强调色（波形/提示文字）：取配色最亮端
fn accent_for(cs: ColorPref) -> Color {
    let raw = match cs {
        ColorPref::Preset(i) => STOPS[i % settings::PRESET_COUNT].to_vec(),
        ColorPref::Custom { r, g, b } => custom_ramp(r, g, b, 8),
    };
    let c = raw.last().copied().unwrap_or([34, 211, 238]);
    Color::Rgb(c[0], c[1], c[2])
}

fn color_label(cs: ColorPref) -> String {
    match cs {
        ColorPref::Preset(i) => format!(
            "{} {}/{}",
            settings::preset_name(i % settings::PRESET_COUNT),
            (i % settings::PRESET_COUNT) + 1,
            settings::PRESET_COUNT
        ),
        ColorPref::Custom { r, g, b } => format!("#{:02x}{:02x}{:02x}", r, g, b),
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let h = area.height as usize;
    if h < 12 {
        return;
    }
    let viz_h = ((h / 3).clamp(4, 14)) as u16;
    let chunks = Layout::vertical([
        Constraint::Length(1),     // 标题/路径
        Constraint::Length(2),     // 播放状态
        Constraint::Min(2),        // 列表
        Constraint::Length(1),     // 进度
        Constraint::Length(viz_h), // 可视化
        Constraint::Length(2),     // 帮助
    ])
    .split(area);

    // 标题：当前目录
    let path_line = Line::from(vec![
        Span::styled(
            " TMus ",
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.cwd.to_string_lossy().into_owned(),
            Style::default().fg(DIM_C),
        ),
    ]);
    f.render_widget(Paragraph::new(path_line), chunks[0]);

    // 播放状态（两行：状态行 + 可视化设置行）
    let (cur_path, playing, pos, dur) = app.engine.snapshot();
    let cur_name = Path::new(&cur_path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let state_mark = if app.loading {
        "解码中…"
    } else if cur_path.is_empty() {
        "就绪"
    } else if playing {
        "▶ 播放中"
    } else {
        "⏸ 已暂停"
    };
    let accent = accent_for(app.settings.color);
    let status_lines = vec![
        Line::from(vec![
            Span::styled(
                format!(" {state_mark}  "),
                Style::default().fg(PROG_C).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                if app.loading {
                    app.notice.clone().unwrap_or_default()
                } else {
                    cur_name.clone()
                },
                Style::default().fg(NOW_C),
            ),
        ]),
        Line::from(vec![
            Span::styled("  样式 ", Style::default().fg(DIM_C)),
            Span::styled(app.settings.viz.label(), Style::default().fg(accent)),
            Span::styled(" ｜ 配色 ", Style::default().fg(DIM_C)),
            Span::styled(color_label(app.settings.color), Style::default().fg(accent)),
            Span::styled(
                "   [v 切换样式 · c 换配色 · C 自定义]",
                Style::default().fg(DIM_C),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(status_lines), chunks[1]);

    // 列表（手动滚动窗口）
    let list_area = chunks[2];
    let viewport = list_area.height as usize;
    let len = app.rows.len();
    if len > 0 {
        let sel = app.sel.min(len - 1);
        let mut top = if sel >= viewport {
            sel - viewport + 1
        } else {
            0
        };
        top = top.min(len.saturating_sub(viewport));
        let rows: Vec<Line> = (top..len.min(top + viewport))
            .map(|i| row_line(app, i, &cur_path))
            .collect();
        f.render_widget(Paragraph::new(rows), list_area);
    } else {
        let empty = Paragraph::new("（此目录没有音频文件）")
            .style(Style::default().fg(DIM_C))
            .alignment(Alignment::Center);
        f.render_widget(empty, list_area);
    }

    // 进度条
    let progress_line = if cur_path.is_empty() {
        Line::from("")
    } else {
        let pct = if dur > 0.0 { (pos / dur).clamp(0.0, 1.0) } else { 0.0 };
        let bar_w = chunks[3].width as usize;
        let bar_w = bar_w.saturating_sub(24).min(60);
        let filled = (pct * bar_w as f64) as usize;
        let bar: String = "━".repeat(filled) + &"─".repeat(bar_w.saturating_sub(filled));
        Line::from(vec![
            Span::styled(
                format!(" {} / {} ", fmt_time(pos), fmt_time(dur)),
                Style::default().fg(DIM_C),
            ),
            Span::styled(bar, Style::default().fg(PROG_C)),
        ])
    };
    f.render_widget(Paragraph::new(progress_line), chunks[3]);

    // 可视化
    let viz_lines = render_viz(chunks[4], app);
    f.render_widget(Paragraph::new(viz_lines), chunks[4]);

    // 帮助（两行）
    let help1 = Line::from(vec![Span::styled(
        " ↑/↓ 选择 · Enter 播放/进目录 · Space 暂停 · n/p 切歌 · ←/→ ±5s · Backspace 上级 · r 刷新 · x 停止 · q 退出",
        Style::default().fg(DIM_C),
    )]);
    let help2 = Line::from(vec![
        Span::styled(" 可视化 ", Style::default().fg(DIM_C)),
        Span::styled("[v]", Style::default().fg(accent)),
        Span::styled(" 切换方式(频谱柱/波形/环形频谱)  ", Style::default().fg(DIM_C)),
        Span::styled("[c]", Style::default().fg(accent)),
        Span::styled(" 循环配色  ", Style::default().fg(DIM_C)),
        Span::styled("[C]", Style::default().fg(accent)),
        Span::styled(" 自定义 #rrggbb", Style::default().fg(DIM_C)),
    ]);
    f.render_widget(Paragraph::new(vec![help1, help2]), chunks[5]);

    // 输入弹层
    if let Some(p) = &app.prompt {
        draw_prompt(f, area, p);
    }
}

fn draw_prompt(f: &mut Frame, area: Rect, p: &Prompt) {
    let w = area.width.min(64);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + area.height.saturating_sub(6) / 2;
    let box_area = Rect::new(x, y, w, 4);
    let head = Line::from(Span::styled(
        p.label.clone(),
        Style::default().fg(Color::LightCyan),
    ));
    let input = Line::from(vec![
        Span::styled("  ", Style::default().fg(Color::White)),
        Span::styled(p.buf.clone(), Style::default().fg(Color::White)),
        Span::styled("▏", Style::default().fg(Color::LightGreen)),
        Span::styled(" ".repeat(7usize.saturating_sub(p.buf.len() + 1)), Style::default().fg(Color::DarkGray)),
    ]);
    let par = Paragraph::new(vec![head, Line::from(""), input])
        .block(Block::default().borders(Borders::ALL).title(" 自定义颜色 "))
        .style(Style::default().bg(Color::Rgb(20, 22, 32)));
    f.render_widget(par, box_area);
}

fn row_line(app: &App, index: usize, cur_path: &str) -> Line<'static> {
    let Some(row) = app.rows.get(index) else {
        return Line::from("");
    };
    let selected = index == app.sel;
    let is_now = row
        .path()
        .map(|p| p.to_string_lossy().into_owned() == cur_path)
        .unwrap_or(false);

    let (prefix, color): (&str, Color) = match row {
        Row::Parent => ("↑", DIM_C),
        Row::Dir { .. } => ("D ", DIR_C),
        Row::File { .. } => ("♪ ", FILE_C),
    };
    let marker = if is_now { "● " } else { "  " };

    let base = Style::default();
    let span_style = |s: Style, c: Color, b: bool| {
        let mut st = s.fg(if b { NOW_C } else { c });
        if selected || b {
            st = st.add_modifier(Modifier::BOLD);
        }
        if selected {
            st = st.bg(SEL_BG);
        }
        st
    };

    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(marker, span_style(base, NOW_C, false)));
    spans.push(Span::styled(prefix, span_style(base, color, is_now)));
    spans.push(Span::styled(
        row.name(),
        span_style(base, if is_now { NOW_C } else { color }, is_now),
    ));
    if let Row::File { size, .. } = row {
        spans.push(Span::styled(
            format!("  {}", fmt_size(*size)),
            span_style(base, DIM_C, is_now),
        ));
    }
    Line::from(spans)
}

// ---------------- 三种可视化绘制（返回文本行，由 render_viz 渲染） ----------------

fn render_viz(area: Rect, app: &mut App) -> Vec<Line<'static>> {
    let (w, h) = (area.width as usize, area.height as usize);
    if w < 8 || h < 2 {
        return vec![Line::from("")];
    }
    let Some((samples, pos)) = app.engine.viz_frame() else {
        return vec![Line::from(
            Span::styled(
                "（播放后这里会显示可视化）",
                Style::default().fg(DIM_C),
            ),
        )];
    };

    let cs = app.settings.color;
    match app.settings.viz {
        VizKind::Bars => {
            let bands = w.min(96).max(8);
            app.spec.set_bands(bands);
            let levels = app.spec.analyze(&samples, pos, app.engine.device_rate);
            draw_bars(area, &levels, &palette_for(cs, 8))
        }
        VizKind::Ring => {
            let bands = w.min(64).max(16);
            app.spec.set_bands(bands);
            let levels = app.spec.analyze(&samples, pos, app.engine.device_rate);
            draw_ring(area, &levels, &palette_for(cs, 12))
        }
        VizKind::Wave => draw_wave(area, &samples, pos, app.engine.device_rate, accent_for(cs)),
    }
}

/// 频谱柱（自底向上填充）
fn draw_bars(area: Rect, levels: &[f32], palette: &[Color]) -> Vec<Line<'static>> {
    let (w, h) = (area.width as usize, area.height as usize);
    let bands = levels.len().max(1);
    let height = h as f32;
    let mut lines: Vec<Line> = (0..h)
        .map(|y| {
            let mut spans = Vec::with_capacity(w);
            for (c, &lvl) in levels.iter().enumerate() {
                let col_w = if c == bands - 1 {
                    w.saturating_sub(bands - 1).max(1)
                } else {
                    1
                };
                let filled = (lvl * height).round() as usize;
                let from_bottom = h - 1 - y;
                let color = palette[(c * palette.len()) / bands];
                let ch = if from_bottom < filled { "█" } else { " " };
                spans.extend(
                    (0..col_w).map(|_| Span::styled(ch, Style::default().fg(color))),
                );
            }
            while spans.len() < w {
                spans.push(Span::raw(" "));
            }
            Line::from(spans)
        })
        .collect();
    lines.reverse(); // 顶部留空、底部填充
    lines
}

/// 波形（时域示波器，单线描边）
fn draw_wave(
    area: Rect,
    samples: &[i16],
    pos: usize,
    rate: u32,
    color: Color,
) -> Vec<Line<'static>> {
    let (w, h) = (area.width as usize, area.height as usize);
    let frames = samples.len() / 2;
    if frames < 64 {
        return vec![Line::from("")];
    }
    let win_raw = rate as usize / 12; // ~83ms
    let win = win_raw.min(frames).max(64);
    let start = {
        let s = pos.saturating_sub(win);
        if s + win <= frames {
            s
        } else {
            frames - win
        }
    };

    // 每列在窗口内取一个值（列间轻微平滑：前后 1 帧平均）
    let center = (h as f32 - 1.0) / 2.0;
    let scale = ((h as f32) / 2.0 - 1.0).max(1.0);
    let mut rows_at = vec![0usize; w];
    for c in 0..w {
        let fi = start + (c * (win - 1)) / w.max(1);
        let l = samples[fi * 2] as f32;
        let r = samples[fi * 2 + 1] as f32;
        let (l2, r2) = if fi + 1 < frames {
            (samples[(fi + 1) * 2] as f32, samples[(fi + 1) * 2 + 1] as f32)
        } else {
            (l, r)
        };
        let v = ((l + r) * 0.5 + (l2 + r2) * 0.5) * 0.5 / 32768.0;
        let row = (center - v * scale).round().clamp(0.0, (h - 1) as f32) as usize;
        rows_at[c] = row;
    }

    let axis = center.round() as usize;
    (0..h)
        .map(|y| {
            let spans: Vec<Span> = (0..w)
                .map(|c| {
                    let ch = if y == rows_at[c] {
                        "█"
                    } else if y == axis && c % 6 == 0 {
                        "·"
                    } else {
                        " "
                    };
                    let st = if y == rows_at[c] {
                        Style::default().fg(color)
                    } else if y == axis {
                        Style::default().fg(DIM_C)
                    } else {
                        Style::default()
                    };
                    Span::styled(ch, st)
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

/// 环形频谱（雷达式：按角度映射频段，幅度决定半径）
fn draw_ring(area: Rect, levels: &[f32], palette: &[Color]) -> Vec<Line<'static>> {
    let (w, h) = (area.width as usize, area.height as usize);
    let cx = (w as f32 - 1.0) / 2.0;
    let cy = (h as f32 - 1.0) / 2.0;
    let max_r = cx.min(cy).max(2.0);
    let inner = (max_r * 0.3).max(0.8);
    let bands = levels.len().max(1);
    let tau = std::f64::consts::TAU;

    let mut lines = Vec::with_capacity(h);
    for y in 0..h {
        let spans: Vec<Span> = (0..w)
            .map(|x| {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let r = (dx * dx + dy * dy).sqrt();
                if r < 0.5 {
                    return Span::raw(" ");
                }
                // 淡淡的基准内圈
                if (r - inner).abs() < 0.6 {
                    return Span::styled("·", Style::default().fg(DIM_C));
                }
                let angle = (dy as f64).atan2(dx as f64) + std::f64::consts::PI; // 0..2PI
                let b = ((angle / tau * bands as f64).round() as usize).min(bands - 1);
                let grow = levels[b].clamp(0.0, 1.0);
                let expected = inner + grow * (max_r - inner);
                if (r - expected).abs() < 0.55 {
                    let color = palette[(b * palette.len()) / bands];
                    Span::styled("█", Style::default().fg(color))
                } else {
                    Span::raw(" ")
                }
            })
            .collect();
        lines.push(Line::from(spans));
    }
    lines
}

// ---------------- 主流程 ----------------

/// Windows：把控制台输出切到 UTF-8(65001)，避免中文与方块字符乱码
#[cfg(windows)]
fn init_console_utf8() {
    use windows_sys::Win32::System::Console::SetConsoleOutputCP;
    unsafe {
        let _ = SetConsoleOutputCP(65001);
    }
}

struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut out = io::stdout();
        let _ = out.execute(LeaveAlternateScreen);
        let _ = out.flush();
    }
}

fn main() -> io::Result<()> {
    #[cfg(windows)]
    init_console_utf8();

    use std::io::IsTerminal;
    if !io::stdin().is_terminal() {
        eprintln!("[提示] tmus 需要在真实终端（TTY）中运行。");
        eprintln!("       请打开“终端”或 iTerm2，进入音乐文件夹后执行 tmus");
        std::process::exit(1);
    }

    let start_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);

    let (engine, dev) = match Engine::new() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[错误] 无法初始化音频: {e}");
            std::process::exit(1);
        }
    };

    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        old_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(LeaveAlternateScreen)?;
    let _restore = Restore;

    let settings = settings::load();
    let mut app = App::new(engine, settings);
    app.cd(&start_dir);

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    terminal.clear()?;

    let mut last = std::time::Instant::now();

    'main: loop {
        app.check_loader();

        terminal.draw(|f| ui(f, &mut app))?;

        if app.quit {
            break 'main;
        }

        // 固定节奏刷新（可视化动画 ~30fps）
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(last);
        let wait = Duration::from_millis(33).saturating_sub(elapsed);
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
        last = std::time::Instant::now();

        if event::poll(Duration::from_millis(0))? {
            if let Event::Key(k) = event::read()? {
                if k.kind == KeyEventKind::Press || k.kind == KeyEventKind::Repeat {
                    app.handle_key(k.code);
                    if app.quit {
                        break 'main;
                    }
                }
            }
        }
    }

    terminal.show_cursor()?;
    disable_raw_mode()?;
    println!("音频设备: {dev}");
    println!("tmus 已退出。");
    Ok(())
}
