use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct File {
    path: PathBuf,
    content: String,
}

#[derive(Debug, Clone)]
pub struct Fragment {
    pub path: PathBuf,
    pub first_line: usize,
    pub content: String,
}

impl File {
    fn read<P: AsRef<Path>>(file: P) -> anyhow::Result<Self> {
        let path = file.as_ref().to_path_buf();
        let content = std::fs::read_to_string(file)?;
        Ok(Self { path, content })
    }

    pub fn into_fragments(self, lines_per_block: usize, blocks_per_fragment: usize) -> anyhow::Result<Vec<Fragment>> {
        let lines = self.content.lines().enumerate().collect::<Vec<_>>();
        let blocks = lines.chunks(lines_per_block).collect::<Vec<_>>();
        let fragments = blocks.windows(blocks_per_fragment).map(|window| {
            let path = self.path.clone();
            let first_line = window[0][0].0;
            let content = window.into_iter().flat_map(|&block| block.into_iter().map(|(_, line)| line)).cloned().collect::<Vec<_>>().join("\n");

            Fragment {path, first_line, content}
        }).collect::<Vec<_>>();
        Ok(fragments)
    }
}

pub fn file_to_fragments<P: AsRef<Path>>(file: P, lines_per_block: usize, blocks_per_fragment: usize) -> anyhow::Result<Vec<Fragment>> {
    File::read(file)?.into_fragments(lines_per_block, blocks_per_fragment)
}