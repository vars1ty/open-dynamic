use atomic_refcell::AtomicRefCell;
use indexmap::IndexMap;
use rune::{runtime::SyncFunction, Value};
use std::{rc::Rc, sync::Arc};

/// ImGui widget type.
#[derive(Clone)]
pub enum WidgetType {
    Label(String, usize),
    LabelCustomFont(String, Arc<String>),
    Button(String, Rc<SyncFunction>, Rc<Option<Value>>),
    Spacing(f32, f32),
    Separator,
    F32Slider(String, f32, f32, f32, Rc<SyncFunction>, Rc<Option<Value>>),
    I32Slider(String, i32, i32, i32, Rc<SyncFunction>, Rc<Option<Value>>),
    NextWidgetWidth(f32),
    SameLine,
    Image(String, f32, f32, bool, bool, String),
    InputTextMultiLine(String, String, f32, f32),

    /// Advanced widget which hosts more complex widgets, like collapsing headers.
    SubWidget(
        SubWidgetType,
        IndexMap<String, Arc<AtomicRefCell<WidgetType>>>,
        Rc<SyncFunction>,
        Rc<Option<Value>>,
    ),

    Checkbox(String, bool, Rc<SyncFunction>, Rc<Option<Value>>),
}

/// Sub-widget type, aka types like collapsing headers and alike.
#[derive(Clone)]
pub enum SubWidgetType {
    CenteredWidgets(Option<f32>, [f32; 2]),
    CollapsingHeader(String),
}
