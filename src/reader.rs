use tokio::io::AsyncReadExt;

pub struct CavaReader<R> {
    input: R,
    buf: BarFrame,
}

#[derive(Debug, Copy, Clone)]
pub enum CavaOutputFormat {
    U8,
    U16,
}

/// Represents a state of bars
#[derive(Debug, Clone)]
pub struct BarFrame {
    data: Vec<u8>,
    format: CavaOutputFormat,
}

impl<R: tokio::io::AsyncRead + Unpin> CavaReader<R> {
    pub fn new(format: CavaOutputFormat, num_bars: usize, input: R) -> Self {
        Self {
            input,
            buf: BarFrame::new(format, num_bars),
        }
    }

    /// Reads the next [BarFrame]
    pub async fn next_frame(&mut self) -> Result<&BarFrame, CavaReaderError> {
        match self.input.read_exact(&mut self.buf.data).await {
            Ok(_) => Ok(&self.buf),
            Err(err) => Err(err.into()),
        }
    }
}

impl BarFrame {
    fn new(format: CavaOutputFormat, num_bars: usize) -> Self {
        Self {
            data: vec![0; num_bars * format.bytes_per_bar()],
            format,
        }
    }

    pub fn iter(&self) -> BarFrameIter<'_> {
        BarFrameIter::new(self)
    }

    pub fn num_bars(&self) -> usize {
        self.data.len() / self.format.bytes_per_bar()
    }
}

pub struct BarFrameIter<'a> {
    data: &'a BarFrame,
    pos: usize,
}

pub type Bar = f32;

impl<'a> BarFrameIter<'a> {
    fn new(data: &'a BarFrame) -> Self {
        Self { data, pos: 0 }
    }
}

impl Iterator for BarFrameIter<'_> {
    type Item = Bar;

    fn next(&mut self) -> Option<Self::Item> {
        let bar_height = match self.data.format {
            CavaOutputFormat::U8 => self
                .data
                .data
                .get(self.pos)
                .map(|height| *height as f32 / u8::MAX as f32),
            CavaOutputFormat::U16 => {
                let index = self.pos * 2;
                self.data.data.get(index..=(index + 1)).map(|height| {
                    let height = ((height[1] as u16) << 8) | height[0] as u16;
                    height as f32 / u16::MAX as f32
                })
            }
        };

        self.pos += 1;

        bar_height
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.data.num_bars(), Some(self.data.num_bars()))
    }
}

impl CavaOutputFormat {
    fn bytes_per_bar(self) -> usize {
        match self {
            CavaOutputFormat::U8 => 1,
            CavaOutputFormat::U16 => 2,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CavaReaderError {
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
}
