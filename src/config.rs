use std::{
    io::{Cursor, Write},
    str::FromStr,
};

use ini::Ini;

use crate::reader::CavaOutputFormat;

#[derive(Clone, Default)]
pub struct CavaConfig {
    pub(crate) num_bars: usize,
    pub(crate) output_format: CavaOutputFormat,

    config: String,
}

impl CavaConfig {
    pub fn with_num_bars(self, num_bars: usize) -> Self {
        Self { num_bars, ..self }
    }

    pub fn with_output_format(self, output_format: CavaOutputFormat) -> Self {
        Self {
            output_format,
            ..self
        }
    }

    /// Set the configuration with a [String].
    /// This won't check if `config` is valid.
    /// Will try to update `num_bars` and `output_format`
    pub fn with_config_str(mut self, config: &str) -> Self {
        if let Ok(conf) = Ini::load_from_str(config) {
            self.read_core_config(&conf);
        }
        Self {
            config: config.into(),
            ..self
        }
    }

    /// Set the configuration with a [ini::Ini] reference.
    /// This will update `num_bars` and `output_format` with they are defined by the given config
    pub fn with_config_ini(mut self, config: &Ini) -> Self {
        let mut config_bytes = Vec::new();
        config
            .write_to(&mut config_bytes)
            .expect("failed to write config");

        self.read_core_config(config);

        Self { ..self }
    }

    pub(crate) fn write_to<W>(&self, w: &mut W) -> std::io::Result<()>
    where
        W: std::io::Write,
    {
        let mut conf = Ini::new();
        self.write_core_config(&mut conf);
        write!(w, "{}\n\n", self.config)?;

        conf.write_to(w)?;

        Ok(())
    }

    fn read_core_config(&mut self, config: &Ini) {
        if let Some(num_bars) = config
            .get_from(Some("general"), "bars")
            .and_then(|k| usize::from_str(k).ok())
        {
            self.num_bars = num_bars;
        }

        if let Some(bit_format) = config.get_from(Some("output"), "bit_format") {
            const U8: &str = CavaOutputFormat::U8.to_cava_config_key();
            const U16: &str = CavaOutputFormat::U16.to_cava_config_key();
            match bit_format {
                U8 => self.output_format = CavaOutputFormat::U8,
                U16 => self.output_format = CavaOutputFormat::U16,
                _ => {}
            }
        }
    }

    fn write_core_config(&self, conf: &mut Ini) {
        conf.with_section(Some("general"))
            .set("bars", self.num_bars.to_string());
        conf.with_section(Some("output"))
            .set("method", "raw")
            .set("data_fromat", "binary")
            .set("bit_format", self.output_format.to_cava_config_key());
    }
}

mod test {
    use crate::{config::CavaConfig, reader::CavaOutputFormat};

    fn config_to_string(conf: &CavaConfig) -> String {
        let mut bytes = Vec::new();
        conf.write_to(&mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn config() {
        let cava_config = CavaConfig::default()
            .with_output_format(CavaOutputFormat::U8)
            .with_num_bars(10);

        assert_eq!(
            config_to_string(&cava_config),
            r#"

[general]
bars=10

[output]
method=raw
data_fromat=binary
bit_format=8bit
"#
        );

        let cava_config = cava_config.with_config_str(
            r#"[general]
bars=5

[output]
bit_format=16bit"#,
        );

        assert_eq!(
            config_to_string(&cava_config),
            r#"[general]
bars=5

[output]
bit_format=16bit

[general]
bars=5

[output]
method=raw
data_fromat=binary
bit_format=16bit
"#
        );
    }
}
