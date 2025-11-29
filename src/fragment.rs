use std::path::{Path, PathBuf};

use crate::tui::{SyntectTheme, Theme};
use ratatui::text::{Line, Span};
use std::sync::Arc;
use syntect::{easy::HighlightLines, parsing::SyntaxSet, util::LinesWithEndings};
use syntect_tui::into_span;

#[derive(Debug, Clone)]
struct FileLine {
    line: String,
    highlighted_line: Line<'static>,
}

#[derive(Debug, Clone)]
struct File {
    path: PathBuf,
    content: Vec<FileLine>,
}

#[derive(Debug, Clone)]
pub struct Fragment {
    first_line: usize,
    last_line: usize,
    file: Arc<File>,
}

impl File {
    fn read<P: AsRef<Path>>(file: P, theme: SyntectTheme) -> anyhow::Result<Self> {
        let path = file.as_ref().to_path_buf();
        let content = std::fs::read_to_string(file)?;

        let ext = path.extension().unwrap_or_default();

        let ps = SyntaxSet::load_defaults_newlines();

        let syntax = ps.find_syntax_by_extension(ext.to_str().unwrap()).unwrap();

        let mut highlight = HighlightLines::new(syntax, &theme);

        let lines = content.lines();

        let highlighted_lines =
            LinesWithEndings::from(&content).flat_map(|line| -> anyhow::Result<Line> {
                Ok(Line::from_iter(
                    highlight
                        .highlight_line(line, &ps)?
                        .into_iter()
                        .filter_map(|segment| {
                            into_span(segment)
                                .ok()
                                .map(|span| Span::styled(span.content.into_owned(), span.style))
                        }),
                ))
            });

        let merged: Vec<_> = lines
            .zip(highlighted_lines)
            .map(|(line, highlighted_line)| FileLine {
                line: line.into(),
                highlighted_line,
            })
            .collect();

        let result = Self {
            path,
            content: merged,
        };

        Ok(result)
    }

    pub fn into_fragments(
        self,
        lines_per_block: usize,
        blocks_per_fragment: usize,
    ) -> Vec<Fragment> {
        let file = Arc::new(self);

        let num_lines = file.content.len();
        let start_lines = (0..num_lines).step_by(lines_per_block);

        start_lines
            .map(|first_line| {
                let last_line = std::cmp::min(
                    first_line + lines_per_block * blocks_per_fragment,
                    num_lines - 1,
                );
                Fragment {
                    file: file.clone(),
                    first_line,
                    last_line,
                }
            })
            .collect()
    }
}

impl Fragment {
    fn content_iter(&self) -> impl Iterator<Item = &FileLine> {
        self.file
            .content
            .iter()
            .skip(self.first_line)
            .take(self.last_line - self.first_line + 1)
    }
    pub fn content(&self) -> String {
        self.content_iter()
            .map(|c| c.line.as_ref())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn location(&self) -> String {
        format!("{}:{}", self.file.path.display(), self.first_line)
    }

    pub fn highlighted_content(&self) -> Vec<Line<'static>> {
        self.content_iter()
            .map(|c| c.highlighted_line.clone())
            .collect::<Vec<_>>()
    }
}

pub fn file_to_fragments<P: AsRef<Path>>(
    file: P,
    lines_per_block: usize,
    blocks_per_fragment: usize,
    theme: Theme,
) -> anyhow::Result<Vec<Fragment>> {
    let theme: SyntectTheme = theme.into();
    Ok(File::read(file, theme)?.into_fragments(lines_per_block, blocks_per_fragment))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn file_to_fragments_splits_content() -> anyhow::Result<()> {
        let theme = Theme::synthwave();
        let dir = tempdir()?;
        let file_path = dir.path().join("sample.rs");
        std::fs::write(&file_path, "fn one() {}\nfn two() {}\nfn three() {}\n")?;

        let fragments = file_to_fragments(&file_path, 2, 1, theme)?;

        assert_eq!(fragments.len(), 2);
        assert_eq!(
            fragments[0].content(),
            "fn one() {}\nfn two() {}\nfn three() {}"
        );
        assert_eq!(fragments[1].content(), "fn three() {}");
        Ok(())
    }
}
