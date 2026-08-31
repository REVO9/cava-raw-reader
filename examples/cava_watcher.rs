use std::time::Duration;

use cava_raw_reader::{CavaConfig, CavaHandle};
use crossterm::terminal;

#[tokio::main]
pub async fn main() {
    let cava_config = CavaConfig::default().with_num_bars(20);

    let cava = CavaHandle::new(cava_config).unwrap();
    let mut cava = cava_raw_reader::watcher::CavaWatcher::spawn(cava);

    loop {
        let bars = cava.latest_frame().unwrap();

        // clear terminal
        crossterm::execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All)).unwrap();

        let width = terminal::size().map(|(w, _)| w).unwrap_or(30);

        // print each bar
        for bar in bars.iter() {
            let length = (bar * width as f32) as usize;
            let string: String = std::iter::repeat_n('━', length).collect();
            println!("{}", string)
        }

        drop(bars);

        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}
