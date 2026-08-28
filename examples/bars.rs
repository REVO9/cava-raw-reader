use std::process::Stdio;

use cava_raw_reader::reader::{CavaOutputFormat, CavaReader};
use crossterm::terminal;

#[tokio::main]
pub async fn main() {
    let mut cava = tokio::process::Command::new("cava")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let cava_stdout = cava.stdout.take().unwrap();

    let mut reader = CavaReader::new(CavaOutputFormat::U16, 10, cava_stdout);
    loop {
        let bars = reader.next_frame().await.unwrap();
        crossterm::execute!(std::io::stdout(), terminal::Clear(terminal::ClearType::All)).unwrap();
        let width = terminal::size().map(|(w, _)| w).unwrap_or(30);
        for bar in bars.iter() {
            let length = (bar * width as f32) as usize;
            let string: String = std::iter::repeat_n('━', length).collect();
            println!("{}", string)
        }
    }
}
