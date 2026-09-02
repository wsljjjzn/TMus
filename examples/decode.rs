//! 冒烟测试 2：ffmpeg 解码 → 16bit 内存样本（验证 32-bit FLAC 等）。
//!
//!   cargo run --example decode -- <音频文件>

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("用法: cargo run --example decode -- <音频文件>");
        std::process::exit(2);
    };
    let p = std::path::PathBuf::from(&path);
    match tmus::audio::load_playable(&p, 44100) {
        Ok(data) => {
            let secs = data.frames as f64 / 44100.0;
            println!(
                "解码成功: {path}\n  立体声帧: {} | 时长: {:.1} 秒 | 样本内存: {:.1} MB",
                data.frames,
                secs,
                data.samples.len() as f64 * 2.0 / 1048576.0
            );
        }
        Err(e) => {
            eprintln!("解码失败: {e}");
            std::process::exit(1);
        }
    }
}
