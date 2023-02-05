use cursive::views::LayerPosition;

trait LayerPositionExt {
    fn get_index(&self, len: usize) -> Option<usize>;
}

impl LayerPositionExt for LayerPosition {
    fn get_index(&self, len: usize) -> Option<usize> {
        match self {
            LayerPosition::FromBack(i) => Some(*i),
            LayerPosition::FromFront(i) => len.checked_sub(i + 1),
        }
    }
}
