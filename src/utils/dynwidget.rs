use indexmap::IndexMap;
use rune::runtime::Function;
use std::{rc::Rc, sync::Arc};

/// ImGui widget type.
#[derive(Clone)]
pub enum WidgetType {
    Label(String, usize),
    LabelCustomFont(String, Arc<String>),
    Button(String, Rc<Function>),
    LegacyButton(String, String),
    Spacing(f32, f32),
    Separator,
    F32Slider(String, f32, f32, f32),
    I32Slider(String, i32, i32, i32),
    NextWidgetWidth(f32),
    SameLine,
    Image(String, f32, f32, bool, bool, String),
    CenteredWidgets(IndexMap<String, Rc<WidgetType>>, Option<f32>, [f32; 2]),
    InputTextMultiLine(String, String, f32, f32),
}
