use ratatui::layout::Rect;
use tachyonfx::RefRect;

#[derive(Debug)]
pub struct FxFilter {
    main_rects: Vec<RefRect>,
    assign_idx: usize,
    filter: tachyonfx::CellFilter,
}

impl FxFilter {
    pub fn new(size: usize) -> Self {
        let mut main_rects = Vec::with_capacity(size);
        for _ in 0..size {
            main_rects.push(RefRect::new(Rect::ZERO))
        }

        let filter = tachyonfx::CellFilter::AnyOf(
            main_rects
                .iter()
                .cloned()
                .map(|r| tachyonfx::CellFilter::RefArea(r))
                .collect(),
        );

        Self {
            main_rects,
            assign_idx: 0,
            filter,
        }
    }

    pub fn main_filter(&self) -> tachyonfx::CellFilter {
        self.filter.clone()
    }

    pub fn border_filter(&self) -> tachyonfx::CellFilter {
        tachyonfx::CellFilter::Not(self.filter.clone().into())
    }

    pub fn reset(&mut self) {
        for rect in &mut self.main_rects {
            rect.set(Rect::ZERO);
        }
        self.assign_idx = 0;
    }

    pub fn assign(&mut self, rect: Rect) -> anyhow::Result<()> {
        self.main_rects.get_mut(self.assign_idx).ok_or(
            anyhow::anyhow!("No more rects available"),
        )?.set(rect);
        self.assign_idx += 1;
        Ok(())
    }
}
