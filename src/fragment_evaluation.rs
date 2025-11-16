use crate::fragment::Fragment;

#[derive(Debug, Clone)]
pub struct FragmentEvaluation {
    pub fragment: Fragment,
    pub value: f32,
}
