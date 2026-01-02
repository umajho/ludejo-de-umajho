pub mod embed_fs_accessor;

pub trait FsAccessor {
    fn name(&self) -> &str;
    fn load_binary(&self, filename: &str) -> anyhow::Result<Vec<u8>>;
    fn load_string(&self, filename: &str) -> anyhow::Result<String>;
}
