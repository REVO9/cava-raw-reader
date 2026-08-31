use tokio::io::{self, AsyncReadExt};

/// Reads cava frames from a reader
pub struct CavaReader<R> {
    input: R,
    buf: BarFrame,
}

#[derive(Debug, Copy, Clone, Default)]
pub enum CavaOutputFormat {
    U8,
    #[default]
    U16,
}

/// Represents a state of bars
#[derive(Debug, Clone, Default)]
pub struct BarFrame {
    data: Vec<u8>,
    format: CavaOutputFormat,
}

impl<R: tokio::io::AsyncRead + Unpin> CavaReader<R> {
    /// Create a new reader. Make sure `format` and `num_bars` are the same here as for cava
    pub fn new(format: CavaOutputFormat, num_bars: usize, input: R) -> Self {
        Self {
            input,
            buf: BarFrame::new(format, num_bars),
        }
    }

    /// Reads the next [BarFrame]
    pub async fn next_frame(&mut self) -> io::Result<Option<&BarFrame>> {
        match self.input.read_exact(&mut self.buf.data).await {
            Ok(_) => Ok(Some(&self.buf)),
            Err(err) if err.kind() == tokio::io::ErrorKind::UnexpectedEof => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    /// Consumes `self` and returns a stream over Owned [BarFrame], cloning each frame.
    /// This makes it slightly more expensive then [CavaReader::next_frame]
    pub fn into_stream(mut self) -> impl futures::Stream<Item = io::Result<BarFrame>> {
        async_stream::try_stream! {
            while let Some(bars) = self.next_frame().await? {
                yield bars.clone();
            }
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

    pub(crate) const fn to_cava_config_key(&self) -> &str {
        match self {
            CavaOutputFormat::U8 => "8bit",
            CavaOutputFormat::U16 => "16bit",
        }
    }
}
