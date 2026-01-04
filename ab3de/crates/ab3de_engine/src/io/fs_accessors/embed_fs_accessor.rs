use crate::io::fs_accessors::FsAccessor;

pub struct EmbedFsAccessor<T: rust_embed::RustEmbed> {
    name: &'static str,
    _marker: std::marker::PhantomData<T>,
}

impl<T: rust_embed::RustEmbed> EmbedFsAccessor<T> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            _marker: std::marker::PhantomData,
        }
    }
}

/// TODO: normalize all dakuten & handakuten. (Otherwise RustEmbed won't find
/// files in some cases on the web build.)
fn normalize_filename(filename: &str) -> String {
    filename.replace("\u{30d4}", "\u{30d2}\u{309a}") // ???
}

impl<T: rust_embed::RustEmbed> FsAccessor for EmbedFsAccessor<T> {
    fn name(&self) -> &str {
        self.name
    }

    fn load_binary(&self, filename: &str) -> anyhow::Result<Vec<u8>> {
        let file = T::get(&normalize_filename(filename))
            .ok_or_else(|| anyhow::anyhow!("Resource not found: #{}/{}", self.name, filename))?;
        Ok(file.data.into_owned())
    }

    fn load_string(&self, filename: &str) -> anyhow::Result<String> {
        let file = T::get(&normalize_filename(filename))
            .ok_or_else(|| anyhow::anyhow!("Resource not found: #{}/{}", self.name, filename))?;
        let s = std::str::from_utf8(&file.data)?;
        Ok(s.to_string())
    }
}
