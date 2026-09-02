//! 冒烟测试 1：初始化音频输出引擎（不发声），验证 cpal 设备可用。
//!
//!   cargo run --example smoke

fn main() {
    let (engine, dev) = tmus::audio::Engine::new().expect("引擎初始化失败");
    println!("音频设备: {dev} | 采样率: {} Hz", engine.device_rate);
    std::thread::sleep(std::time::Duration::from_millis(500));
    println!("smoke OK：cpal 输出流创建并启动成功");
}
