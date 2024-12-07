use atomic_refcell::AtomicRefCell;
use indexmap::IndexMap;
use rune::{runtime::Function, Value};
use std::{rc::Rc, sync::Arc};

/// ImGui widget type.
#[derive(Clone)]
pub enum WidgetType {
    Label(String, usize),
    LabelCustomFont(String, Arc<String>),
    Button(String, Rc<Function>, Option<Value>),
    LegacyButton(String, String),
    Spacing(f32, f32),
    Separator,
    F32Slider(String, f32, f32, f32, Rc<Function>, Option<Value>),
    I32Slider(String, i32, i32, i32, Rc<Function>, Option<Value>),
    NextWidgetWidth(f32),
    SameLine,
    Image(String, f32, f32, bool, bool, String),
    CenteredWidgets(
        IndexMap<String, Arc<AtomicRefCell<WidgetType>>>,
        Option<f32>,
        [f32; 2],
    ),
    InputTextMultiLine(String, String, f32, f32),
}
